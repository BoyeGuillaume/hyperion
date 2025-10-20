//! Compact append-only tree buffer used to encode expressions.
//!
//! Role
//! - Stores a forest of nodes in a contiguous byte buffer using a fixed layout
//!   (opcode, flags, optional 32-bit data, up to 7 child references).
//! - Enables O(1) amortized appends and cheap cloning of small buffers
//!   (via `smallvec` inlined storage up to 32 bytes).
//! - Supports consolidation to reclaim wasted space after in-place updates.
//!
//! Performance
//! - Pushing a node is amortized O(1). Consolidation (when triggered) is O(n) where n is the
//!   current buffer size.
//! - Node references are 16-bit offsets (`u16`), capping a single buffer at 64 KiB; this keeps
//!   encoded expressions compact and pointer-sized across platforms.
//!
//! Safety & invariants
//! - Methods contain debug assertions to detect cycles and out-of-bounds writes in debug builds.
//! - Consolidation invalidates all previous `TreeBufNodeRef` values; only the new ones remain
//!   valid afterwards.
use smallvec::SmallVec;
use stackalloc::stackalloc_uninit;
use std::mem::MaybeUninit;

use crate::utils::staticvec::StaticVec;

/// Opaque handle to a node stored inside a [`TreeBuf`].
///
/// This is a byte offset into the buffer encoded as `u16`. It is only valid with the specific
/// `TreeBuf` that produced it and becomes invalid when the buffer is consolidated.
pub type TreeBufNodeRef = u16;

/// Growable compact buffer of encoded tree nodes.
///
/// Use [`push_node`], [`push_tree`], and [`set_root`] to build trees. Call [`consolidate`] (or
/// let the encoder call `consolite_if_needed`) to reclaim wasted bytes after structural updates.
#[derive(Clone)]
pub struct TreeBuf {
    bytes: SmallVec<[u8; 32]>,
    // Offset of the root node, or 0 if none
    //
    // Notice that the smallest possible node is 2 bytes (1 byte opcode + 1 byte flags), therefore
    // we can never have a valid node at offset 1, we remap the node offset 1 to mean "no root node":
    root_offset: u16,
}

thread_local! {
    static INTERMEDIATE_BUFFER: std::cell::RefCell<Vec<u8>> = std::cell::RefCell::new(vec![0u8; 1024]);
}

impl TreeBuf {
    /// Maximum number of child references allowed per node.
    pub const MAX_NUM_REFERENCES: usize = 7;
    /// Special invalid node reference. Each node must be at least 2 bytes, so offset 1 is never valid.
    pub const INVALID_NODE_REF: TreeBufNodeRef = 1;
    const MAX_NODE_SIZE: usize = 2 + 4 + 7 * 2; // 1 byte opcode + 1 byte flags + 4 bytes data + 7 * 2 bytes references

    fn encode_node<'a>(
        opcode: u8,
        data: Option<u32>,
        references: &'a [TreeBufNodeRef],
    ) -> impl Iterator<Item = u8> + 'a {
        debug_assert!(
            references.len() <= Self::MAX_NUM_REFERENCES,
            "InlineTree nodes can have at most {} references",
            Self::MAX_NUM_REFERENCES
        );

        // Current allocation algorithm is stupid, just appending everything at the end of the buffer. Format
        // of each node is: opcode (1 byte), flag byte (1 byte), data (0 or 4 bytes), references (2 bytes each).
        let flag_byte = ((references.len() as u8) << 1) | data.is_some() as u8;

        std::iter::once(opcode)
            .chain(std::iter::once(flag_byte))
            .chain(data.into_iter().flat_map(|d| d.to_le_bytes().into_iter()))
            .chain(references.iter().flat_map(|r| r.to_le_bytes().into_iter()))
    }

    fn get_root_offset(&self) -> Option<TreeBufNodeRef> {
        if self.root_offset == Self::INVALID_NODE_REF {
            None
        } else {
            Some(self.root_offset)
        }
    }

    fn set_root_offset(&mut self, new_root: Option<TreeBufNodeRef>) {
        debug_assert!(
            new_root != Some(Self::INVALID_NODE_REF),
            "Cannot set root node to offset 1, it is reserved"
        );
        self.root_offset = new_root.unwrap_or(1);
    }

    fn decode_node(
        buffer: &[u8],
        node_ref: TreeBufNodeRef,
    ) -> (
        u8,
        Option<u32>,
        StaticVec<TreeBufNodeRef, { Self::MAX_NUM_REFERENCES }>,
    ) {
        assert!(node_ref != Self::INVALID_NODE_REF, "Invalid node reference");

        let node_ref = node_ref as usize;
        let magic_byte = buffer[node_ref];
        let flag_byte = buffer[node_ref + 1];

        let has_data = (flag_byte & 1) != 0;
        let num_references = (flag_byte >> 1) as usize;

        let data = if has_data {
            let data_bytes = &buffer[node_ref + 2..node_ref + 6];
            Some(u32::from_le_bytes(data_bytes.try_into().unwrap()))
        } else {
            None
        };

        let references = (0..num_references)
            .map(move |i| {
                let start = node_ref + 2 + (has_data as usize * 4) + i * 2;
                let end = start + 2;
                let reference_bytes = &buffer[start..end];
                TreeBufNodeRef::from_le_bytes(reference_bytes.try_into().unwrap())
            })
            .collect();

        (magic_byte, data, references)
    }

    fn consolidate_internal(
        buffer: &[u8],
        mut write_callback: impl FnMut(&[u8], &mut Option<TreeBufNodeRef>),
        root_offset: TreeBufNodeRef,
    ) -> TreeBufNodeRef {
        debug_assert!(
            root_offset as usize + 2 <= buffer.len(),
            "Root offset out of bounds"
        );

        // Description of the algorithm
        //  1. Pop a node from the stack (offset in old buffer)
        //  2. Write data, flag and magic byte to the intermediate buffer of current
        //  3. Estimate size of all of its children, write dummy children to the write_callback
        //  4. Push children to the stack
        let mut stack: SmallVec<[(TreeBufNodeRef, TreeBufNodeRef); 32]> = SmallVec::new(); // Scales with depth of the tree

        // Push root node (from iterator)
        let new_root_offset = {
            let (opcode, data, reference_iter) = Self::decode_node(buffer, root_offset);
            let references: StaticVec<TreeBufNodeRef, { Self::MAX_NUM_REFERENCES }> =
                reference_iter
                    .into_iter()
                    .map(|_| TreeBufNodeRef::MAX)
                    .collect();
            let static_node: StaticVec<u8, { Self::MAX_NODE_SIZE }> =
                Self::encode_node(opcode, data, &references).collect();

            // Finally encode the data into the intermediate buffer
            let mut new_root_offset = None; // None means at the end
            write_callback(&static_node, &mut new_root_offset);
            stack.push((root_offset, new_root_offset.unwrap())); // (old offset, new offset)
            new_root_offset.unwrap()
        };

        while let Some((old_offset, new_offset)) = stack.pop() {
            let (opcode, data, reference_iter) = Self::decode_node(buffer, old_offset);

            let new_references: StaticVec<TreeBufNodeRef, { Self::MAX_NUM_REFERENCES }> =
                reference_iter
                    .into_iter()
                    .map(|t| {
                        // Write the dummy child now
                        let (opcode, data, references) = Self::decode_node(buffer, t);
                        let references: StaticVec<TreeBufNodeRef, { Self::MAX_NUM_REFERENCES }> =
                            references
                                .into_iter()
                                .map(|_| TreeBufNodeRef::MAX)
                                .collect();
                        let dummy_node: StaticVec<u8, { Self::MAX_NODE_SIZE }> =
                            Self::encode_node(opcode, data, &references).collect();

                        let mut child_offset = None; // None means at the end
                        write_callback(&dummy_node, &mut child_offset);
                        stack.push((t, child_offset.unwrap()));
                        child_offset.unwrap()
                    })
                    .collect();

            // Now that we have the new references, we can write the current node with the correct references
            let static_node: StaticVec<u8, { Self::MAX_NODE_SIZE }> =
                Self::encode_node(opcode, data, &new_references).collect();
            let mut child_offset = Some(new_offset);
            write_callback(&static_node, &mut child_offset);
        }

        // Return new root offset
        new_root_offset
    }

    fn consolidate_buffered(&mut self, intermediate_buffer: &mut [u8]) {
        let root_offset = {
            match self.get_root_offset() {
                Some(r) => r,
                None => {
                    self.bytes.clear();
                    return;
                }
            }
        };

        // First pass, we explore the tree in a depth-first manner, starting from the root node, and we write each node
        // to the intermediate buffer, keeping track of the new offsets of each node in a map
        let mut intermediate_buffer_len: usize = 0;

        // Closure that writes to the intermediate buffer and keeps track of new offsets
        let mut write_callback = |data: &[u8], new_offset: &mut Option<TreeBufNodeRef>| {
            let offset = if let Some(o) = new_offset {
                *o as usize
            } else {
                new_offset.replace(intermediate_buffer_len as TreeBufNodeRef);
                intermediate_buffer_len
            };

            // 1. Ensure we have enough space in the intermediate buffer
            debug_assert!(
                offset + data.len() <= intermediate_buffer.len(),
                "Intermediate buffer overflow"
            );

            // 2. Write data to the intermediate buffer
            intermediate_buffer[offset..offset + data.len()].copy_from_slice(data);
            intermediate_buffer_len = intermediate_buffer_len.max(offset + data.len());
        };

        let new_root_offset =
            Self::consolidate_internal(&self.bytes, &mut write_callback, root_offset);
        debug_assert!(
            new_root_offset == 0,
            "New root offset should be 0 after consolidation"
        );
        self.set_root_offset(Some(new_root_offset));

        // Now that we have the intermediate buffer, we can swap it with the current buffer
        self.bytes.clear();
        self.bytes
            .extend_from_slice(&intermediate_buffer[..intermediate_buffer_len]);
    }

    /// Consolidates the tree by removing wasted bytes and reassigning node offsets.
    ///
    /// WARNING: This invalidates all existing TreeNodeRef references!
    pub fn consolidate(&mut self) {
        // If there are no wasted bytes, we don't need to do anything

        if self.bytes.len() >= 512 {
            INTERMEDIATE_BUFFER.with(|buffer_cell| {
                let mut buffer = buffer_cell.borrow_mut();
                if buffer.len() < self.bytes.len() {
                    buffer.resize(self.bytes.len(), 0);
                }
                let buffer = &mut buffer[..self.bytes.len()];
                self.consolidate_buffered(buffer);
            });
        } else {
            stackalloc_uninit(self.bytes.len(), |buffer: &mut [MaybeUninit<u8>]| {
                let buffer = unsafe {
                    std::slice::from_raw_parts_mut(buffer.as_mut_ptr() as *mut u8, buffer.len())
                };
                self.consolidate_buffered(buffer);
            });
        }

        debug_assert!(
            !self.detect_cycle(self.root_offset),
            "Cycle detected in tree after consolidation"
        );
    }

    /// Detect cycles reachable from `start` (debug utility).
    ///
    /// Complexity: O(n) in the number of reachable nodes.
    pub fn detect_cycle(&self, start: TreeBufNodeRef) -> bool {
        let mut visited: SmallVec<[bool; 256]> = SmallVec::from_elem(false, self.bytes.len());
        let mut stack: SmallVec<[TreeBufNodeRef; 32]> = SmallVec::new();

        stack.push(start);

        while let Some(node_ref) = stack.pop() {
            if visited[node_ref as usize] {
                return true;
            }
            visited[node_ref as usize] = true;

            let (_opcode, _data, references) = Self::decode_node(&self.bytes, node_ref);
            for r in references {
                stack.push(r);
            }
        }

        false
    }

    /// Append a single node to the buffer and return its reference.
    ///
    /// Contract
    /// - `opcode`: user-defined tag for the node kind (fits in one byte).
    /// - `data`: optional 32-bit payload.
    /// - `references`: up to [`Self::MAX_NUM_REFERENCES`] child node refs.
    ///
    /// Panics in debug if the buffer would exceed 64 KiB.
    pub fn push_node(
        &mut self,
        opcode: u8,
        data: Option<u32>,
        references: &[TreeBufNodeRef],
    ) -> TreeBufNodeRef {
        let offset = self.bytes.len();
        self.bytes
            .extend(Self::encode_node(opcode, data, references));
        debug_assert!(
            self.bytes.len() <= u16::MAX as usize,
            "InlineTree cannot exceed 65535 bytes"
        );
        debug_assert!(
            !self.detect_cycle(offset as TreeBufNodeRef),
            "Cycle detected in tree"
        );
        offset as TreeBufNodeRef
    }

    /// Copy a node (and its reachable subgraph) from another buffer into this one.
    ///
    /// Returns the new reference in this buffer. Runs in O(k) where k is the number of copied
    /// nodes. Child sharing is not preserved across buffers (each referenced node is copied
    /// exactly once for the reached subgraph).
    pub fn push_tree(&mut self, other: &TreeBuf, other_node_ref: TreeBufNodeRef) -> TreeBufNodeRef {
        // Call to consolidate_callback to copy the other tree into this one in an efficient manner
        Self::consolidate_internal(
            &other.bytes,
            |data: &[u8], new_offset: &mut Option<TreeBufNodeRef>| {
                if let Some(offset) = new_offset {
                    debug_assert!(
                        *offset as usize + data.len() <= self.bytes.len(),
                        "Offset out of bounds"
                    );

                    self.bytes[*offset as usize..*offset as usize + data.len()]
                        .copy_from_slice(data);
                } else {
                    let offset = self.bytes.len();
                    self.bytes.extend_from_slice(data);
                    *new_offset = Some(offset as TreeBufNodeRef);
                }
            },
            other_node_ref,
        )
    }

    /// Decode a node into its components: `(opcode, data, references)`.
    ///
    /// Returns a small fixed-capacity vector for references to avoid heap allocations.
    pub fn get_node(
        &self,
        node_ref: TreeBufNodeRef,
    ) -> (
        u8,
        Option<u32>,
        StaticVec<u16, { Self::MAX_NUM_REFERENCES }>,
    ) {
        Self::decode_node(&self.bytes, node_ref)
    }

    /// Mark a node as the root of the logical tree contained in the buffer.
    ///
    /// Useful after finishing a sequence of pushes.
    pub fn set_root(&mut self, new_ref: TreeBufNodeRef) {
        self.set_root_offset(Some(new_ref));

        debug_assert!(!self.detect_cycle(new_ref), "Cycle detected in tree");
    }

    /// Update the `reference_index`-th child pointer of `node_ref` to `new_ref`.
    ///
    /// Performance
    /// - O(1). Counts previous child as "wasted" bytes to be reclaimed by a future consolidation.
    pub fn update_node_reference(
        &mut self,
        node_ref: TreeBufNodeRef,
        reference_index: u8,
        new_ref: TreeBufNodeRef,
    ) {
        debug_assert!(
            reference_index < 8,
            "InlineTree nodes can have at most 8 references"
        );

        // Encode the new node
        let offset = node_ref as usize;
        let flag_byte = self.bytes[offset + 1];
        let has_data = (flag_byte & 1) != 0;
        let num_references = (flag_byte >> 1) as usize;

        debug_assert!(
            (reference_index as usize) < num_references,
            "Reference index out of bounds"
        );

        let reference_offset =
            offset + 2 + (has_data as usize * 4) + (reference_index as usize) * 2;

        // Find previous reference, count wasted bytes if any
        {
            let previous_reference = TreeBufNodeRef::from_le_bytes(
                self.bytes[reference_offset..reference_offset + 2]
                    .try_into()
                    .unwrap(),
            );

            if previous_reference == new_ref {
                // No change
                return;
            }
        }

        // Update the reference in place
        self.bytes[reference_offset..reference_offset + 2].copy_from_slice(&new_ref.to_le_bytes());

        debug_assert!(
            !self.detect_cycle(node_ref as TreeBufNodeRef),
            "Cycle detected in tree"
        );
    }

    /// Overwrite the 32-bit payload of a node that has one.
    pub fn update_node_data(&mut self, node_ref: TreeBufNodeRef, new_data: u32) {
        let offset = node_ref as usize;
        let flag_byte = self.bytes[offset + 1];
        debug_assert!(
            (flag_byte & 1) != 0,
            "Cannot update data of a node that has no data"
        );

        let data_offset = offset + 2;
        self.bytes[data_offset..data_offset + 4].copy_from_slice(&new_data.to_le_bytes());
    }

    /// Create an empty buffer with inlined capacity.
    pub fn new() -> Self {
        Self {
            bytes: SmallVec::new(),
            root_offset: 1, // No root node
        }
    }

    /// Whether the buffer has a root (and thus contains a logical tree).
    pub fn empty(&self) -> bool {
        self.get_root_offset().is_none()
    }

    /// Total number of bytes currently used by the buffer (including waste).
    pub fn total_bytes(&self) -> usize {
        self.bytes.len()
    }

    /// Return the current root node, if any.
    pub fn root(&self) -> Option<TreeBufNodeRef> {
        self.get_root_offset()
    }
}

impl Default for TreeBuf {
    fn default() -> Self {
        Self::new()
    }
}

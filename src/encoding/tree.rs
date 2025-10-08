use smallvec::SmallVec;
use stackalloc::stackalloc_uninit;
use std::mem::MaybeUninit;

use crate::utils::staticvec::StaticVec;

pub type TreeBufNodeRef = u16;

#[derive(Clone)]
pub struct TreeBuf {
    bytes: SmallVec<[u8; 32]>,
    // Offset of the root node, or 0 if none
    //
    // Notice that the smallest possible node is 2 bytes (1 byte opcode + 1 byte flags), therefore
    // we can never have a valid node at offset 1, we remap the node offset 1 to mean "no root node":
    root_offset: u16,
    // Number of bytes that are wasted due to node updates
    //
    // Notice that this number is overestimated if we duplicate references to the same node, however
    // the cost of reference counting is higher than the cost of additional consolidations
    wasted_bytes: u16,
}

thread_local! {
    static INTERMEDIATE_BUFFER: std::cell::RefCell<Vec<u8>> = std::cell::RefCell::new(vec![0u8; 1024]);
}

impl TreeBuf {
    fn encode_node<'a>(
        opcode: u8,
        data: Option<u32>,
        references: &'a [TreeBufNodeRef],
    ) -> impl Iterator<Item = u8> + 'a {
        debug_assert!(
            references.len() <= 7,
            "InlineTree nodes can have at most 7 references"
        );

        // Current allocation algorithm is stupid, just appending everything at the end of the buffer. Format
        // of each node is: opcode (1 byte), flag byte (1 byte), data (0 or 4 bytes), references (2 bytes each).
        let flag_byte = ((references.len() as u8) << 1) | data.is_some() as u8;

        std::iter::once(opcode as u8)
            .chain(std::iter::once(flag_byte as u8))
            .chain(data.into_iter().flat_map(|d| d.to_le_bytes().into_iter()))
            .chain(references.iter().flat_map(|r| r.to_le_bytes().into_iter()))
    }

    fn get_root_offset(&self) -> Option<TreeBufNodeRef> {
        if self.root_offset == 1 {
            None
        } else {
            Some(self.root_offset)
        }
    }

    fn set_root_offset(&mut self, new_root: Option<TreeBufNodeRef>) {
        debug_assert!(
            new_root != Some(1),
            "Cannot set root node to offset 1, it is reserved"
        );
        self.root_offset = new_root.unwrap_or(1);
    }

    fn decode_node(
        buffer: &[u8],
        node_ref: TreeBufNodeRef,
    ) -> (u8, Option<u32>, StaticVec<TreeBufNodeRef, 8>) {
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

    fn static_buffer_writer(
        buffer: &mut [u8],
        buffer_len: &mut usize,
        iter: impl Iterator<Item = u8>,
    ) {
        for byte in iter {
            buffer[*buffer_len] = byte;
            *buffer_len += 1;
        }
    }

    fn consolidate_buffered(&mut self, intermediate_buffer: &mut [u8]) {
        let root_offset = {
            match self.get_root_offset() {
                Some(r) => r,
                None => {
                    self.bytes.clear();
                    self.wasted_bytes = 0;
                    return;
                }
            }
        };

        // First pass, we explore the tree in a depth-first manner, starting from the root node, and we write each node
        // to the intermediate buffer, keeping track of the new offsets of each node in a map
        let mut intermediate_buffer_len: usize = 0;
        let mut stack: SmallVec<[(TreeBufNodeRef, TreeBufNodeRef); 32]> = SmallVec::new(); // Scales with depth of the tree

        // Push root node (from iterator)
        {
            stack.push((root_offset, 0)); // (old offset, new offset)

            let (opcode, data, reference_iter) = Self::decode_node(&self.bytes, self.root_offset);
            let references: StaticVec<TreeBufNodeRef, 8> = reference_iter
                .into_iter()
                .map(|_| TreeBufNodeRef::MAX)
                .collect();
            let encoder = Self::encode_node(opcode, data, &references);

            // Finally encode the data into the intermediate buffer
            Self::static_buffer_writer(intermediate_buffer, &mut intermediate_buffer_len, encoder);
            self.set_root_offset(Some(0));
        }

        // In the first pass, we allocate each node that is reachable from the root node
        // we proceed in such manner
        // 1. Pop a node from the stack (offset in old buffer)
        // 2. Write data, flag and magic byte to the intermediate buffer of current node
        // 3. Estimate size of all of its children, calculate offsets of each child in the intermediate buffer
        // 4. Push children to the stack
        while let Some((old_offset, new_offset)) = stack.pop() {
            let (opcode, data, reference_iter) = Self::decode_node(&self.bytes, old_offset);

            let new_references: StaticVec<TreeBufNodeRef, 8> = reference_iter
                .into_iter()
                .map(|t| {
                    // Each child will be written at the end of the intermediate buffer
                    let child_offset = intermediate_buffer_len as TreeBufNodeRef;
                    stack.push((t, child_offset));

                    // Write the dummy child now
                    let (opcode, data, references) = Self::decode_node(&self.bytes, t);
                    let references: StaticVec<TreeBufNodeRef, 8> = references
                        .into_iter()
                        .map(|_| TreeBufNodeRef::MAX)
                        .collect();
                    let encoder = Self::encode_node(opcode, data, &references);
                    Self::static_buffer_writer(
                        intermediate_buffer,
                        &mut intermediate_buffer_len,
                        encoder,
                    );

                    child_offset
                })
                .collect();

            // Now that we have the new references, we can write the current node with the correct references
            let encoder = Self::encode_node(opcode, data, &new_references);
            let mut fake_buffer_len = new_offset as usize;
            Self::static_buffer_writer(intermediate_buffer, &mut fake_buffer_len, encoder);
        }

        // Now that we have the intermediate buffer, we can swap it with the current buffer
        self.bytes.clear();
        self.bytes
            .extend_from_slice(&intermediate_buffer[..intermediate_buffer_len as usize]);
        self.wasted_bytes = 0;
    }

    pub fn should_consolidate(&self) -> bool {
        // We consolidate only if we have wasted space, in the case where have not yet
        // spilled over the heap, we consolidate aggressively otherwise we wait until we have
        // at least 25% wasted space
        if self.wasted_bytes == 0 {
            return false;
        }

        if self.bytes.spilled() {
            self.wasted_bytes as usize >= self.bytes.len() / 4
        } else {
            true
        }
    }

    pub fn consolite_if_needed(&mut self) {
        if self.should_consolidate() {
            self.consolidate();
        }
    }

    /// Consolidates the tree by removing wasted bytes and reassigning node offsets.
    ///
    /// WARNING: This invalidates all existing TreeNodeRef references!
    pub fn consolidate(&mut self) {
        // If there are no wasted bytes, we don't need to do anything
        if self.wasted_bytes == 0 {
            return;
        }

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
            self.detect_cycle(self.root_offset) == false,
            "Cycle detected in tree after consolidation"
        );
    }

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
            self.detect_cycle(offset as TreeBufNodeRef) == false,
            "Cycle detected in tree"
        );
        offset as TreeBufNodeRef
    }

    pub fn get_node(&self, node_ref: TreeBufNodeRef) -> (u8, Option<u32>, StaticVec<u16, 8>) {
        Self::decode_node(&self.bytes, node_ref)
    }

    pub fn update_root_node(&mut self, new_ref: TreeBufNodeRef) {
        self.set_root_offset(Some(new_ref));

        debug_assert!(
            self.detect_cycle(new_ref) == false,
            "Cycle detected in tree"
        );
    }

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

            let previous_flag_byte = self.bytes[previous_reference as usize + 1];
            let previous_num_references = (previous_flag_byte >> 1) as usize;
            let previous_node_size =
                2 + (previous_flag_byte & 1) as usize * 4 + previous_num_references * 2;
            self.wasted_bytes += previous_node_size as TreeBufNodeRef;
        }

        // Update the reference in place
        self.bytes[reference_offset..reference_offset + 2].copy_from_slice(&new_ref.to_le_bytes());

        debug_assert!(
            self.detect_cycle(node_ref as TreeBufNodeRef) == false,
            "Cycle detected in tree"
        );
    }

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

    pub fn new() -> Self {
        Self {
            bytes: SmallVec::new(),
            root_offset: 1, // No root node
            wasted_bytes: 0,
        }
    }

    pub fn empty(&self) -> bool {
        self.get_root_offset().is_none()
    }

    pub fn total_bytes(&self) -> usize {
        self.bytes.len()
    }

    pub fn root(&self) -> Option<TreeBufNodeRef> {
        self.get_root_offset()
    }
}

impl Default for TreeBuf {
    fn default() -> Self {
        Self::new()
    }
}

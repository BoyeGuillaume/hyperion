use smallvec::SmallVec;
use stackalloc::stackalloc_uninit;
use std::mem::MaybeUninit;

use crate::utils::staticvec::StaticVec;

pub type TreeNodeRef = u16;

pub struct InlineTree {
    bytes: SmallVec<[u8; 32]>,
    root_offset: u16,
    wasted_bytes: u16,
}

thread_local! {
    static INTERMEDIATE_BUFFER: std::cell::RefCell<Vec<u8>> = std::cell::RefCell::new(vec![0u8; 1024]);
}

impl InlineTree {
    fn encode_node<'a>(
        opcode: u8,
        data: Option<u32>,
        references: &'a [TreeNodeRef],
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

    fn decode_node(
        buffer: &[u8],
        node_ref: TreeNodeRef,
    ) -> (u8, Option<u32>, StaticVec<TreeNodeRef, 8>) {
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
                TreeNodeRef::from_le_bytes(reference_bytes.try_into().unwrap())
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
        // First pass, we explore the tree in a depth-first manner, starting from the root node, and we write each node
        // to the intermediate buffer, keeping track of the new offsets of each node in a map
        let mut intermediate_buffer_len: usize = 0;
        let mut stack: SmallVec<[(TreeNodeRef, TreeNodeRef); 32]> = SmallVec::new(); // Scales with depth of the tree

        // Push root node (from iterator)
        {
            stack.push((self.root_offset, 0)); // (old offset, new offset)

            let (opcode, data, reference_iter) = Self::decode_node(&self.bytes, self.root_offset);
            let references: StaticVec<TreeNodeRef, 8> = reference_iter
                .into_iter()
                .map(|_| TreeNodeRef::MAX)
                .collect();
            let encoder = Self::encode_node(opcode, data, &references);

            // Finally encode the data into the intermediate buffer
            Self::static_buffer_writer(intermediate_buffer, &mut intermediate_buffer_len, encoder);
        }

        // In the first pass, we allocate each node that is reachable from the root node
        // we proceed in such manner
        // 1. Pop a node from the stack (offset in old buffer)
        // 2. Write data, flag and magic byte to the intermediate buffer of current node
        // 3. Estimate size of all of its children, calculate offsets of each child in the intermediate buffer
        // 4. Push children to the stack
        while let Some((old_offset, new_offset)) = stack.pop() {
            let (opcode, data, reference_iter) = Self::decode_node(&self.bytes, old_offset);

            let new_references: StaticVec<TreeNodeRef, 8> = reference_iter
                .into_iter()
                .map(|t| {
                    // Each child will be written at the end of the intermediate buffer
                    let child_offset = intermediate_buffer_len as TreeNodeRef;
                    stack.push((t, child_offset));

                    // Write the dummy child now
                    let (opcode, data, references) = Self::decode_node(&self.bytes, t);
                    let references: StaticVec<TreeNodeRef, 8> =
                        references.into_iter().map(|_| TreeNodeRef::MAX).collect();
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
    }

    pub fn push_node(
        &mut self,
        opcode: u8,
        data: Option<u32>,
        references: &[TreeNodeRef],
    ) -> TreeNodeRef {
        let offset = self.bytes.len();
        self.bytes
            .extend(Self::encode_node(opcode, data, references));
        debug_assert!(
            self.bytes.len() <= u16::MAX as usize,
            "InlineTree cannot exceed 65535 bytes"
        );
        offset as TreeNodeRef
    }

    pub fn get_node(&self, node_ref: TreeNodeRef) -> (u8, Option<u32>, StaticVec<u16, 8>) {
        Self::decode_node(&self.bytes, node_ref)
    }

    pub fn update_node_reference(
        &mut self,
        node: Option<TreeNodeRef>,
        reference_index: u8,
        new_ref: TreeNodeRef,
    ) {
        debug_assert!(
            reference_index < 8,
            "InlineTree nodes can have at most 8 references"
        );

        match node {
            Some(node_ref) => {
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
                self.bytes[reference_offset..reference_offset + 2]
                    .copy_from_slice(&new_ref.to_le_bytes());
            }
            None => {
                debug_assert!(
                    reference_index == 0,
                    "Cannot update reference of None node except at index 0"
                );
                debug_assert!(self.root_offset == 0, "Root node already set");
                self.root_offset = new_ref;
            }
        }
    }

    pub fn update_node_data(&mut self, node_ref: TreeNodeRef, new_data: u32) {
        let offset = node_ref as usize;
        let flag_byte = self.bytes[offset + 1];
        debug_assert!(
            (flag_byte & 1) != 0,
            "Cannot update data of a node that has no data"
        );

        let data_offset = offset + 2;
        self.bytes[data_offset..data_offset + 4].copy_from_slice(&new_data.to_le_bytes());
    }
}

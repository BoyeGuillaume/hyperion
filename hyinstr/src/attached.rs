use std::{collections::BTreeMap, sync::Arc};

use slotmap::{DefaultKey, Key, KeyData, SlotMap};
use smallvec::SmallVec;

use crate::modules::{
    Function, InstructionRef,
    instructions::{HyInstr, Instruction},
    operand::{Label, Name, Operand},
};

/// Represents an meta-function attached to an existing function for theorem derivation.
///
/// This structure allows for the modification and extension of an existing function
/// by overlaying additional instructions and assertions. It maintains its own
/// labeling and naming scheme to avoid conflicts with the target function.
///
#[derive(Debug)]
pub struct AttachedFunction {
    pub target: Arc<Function>,

    next_available_label: Label,
    next_available_name: Name,
    derive_dest_map: BTreeMap<Name, (InstructionRef, u16)>,
    overlay: BTreeMap<Label, SlotMap<DefaultKey, HyInstr>>,
    /// Map from hash to instruction indices in the overlay. Allow to optimize
    /// simple deduplication of instructions. u64 should be the hash of the
    /// instruction with nil destination (if any).
    index_dedup_instr: BTreeMap<u64, SmallVec<InstructionRef, 1>>,
    begin_assert: SlotMap<DefaultKey, HyInstr>,
    end_assert: SlotMap<DefaultKey, HyInstr>,
}

impl AttachedFunction {
    pub const BEGIN_LABEL: Label = Label(u32::MAX - 1);
    pub const END_LABEL: Label = Label(u32::MAX - 2);

    fn compute_hash(instr: &mut HyInstr, label: Label) -> u64 {
        use std::hash::{Hash, Hasher};

        let destination = instr.destination();
        if destination.is_some() {
            instr.set_destination(Name(0)); // Ignore destination for hashing
        }

        let mut hasher = std::hash::DefaultHasher::new();
        instr.hash(&mut hasher);
        label.hash(&mut hasher);

        if let Some(dest_name) = destination {
            instr.set_destination(dest_name); // Restore original destination
        }

        hasher.finish()
    }

    /// Get the next available label for the attached function.
    pub fn next_available_label(&mut self) -> Label {
        assert!(
            self.next_available_label < Self::END_LABEL,
            "Exceeded maximum number of labels available for attached function."
        );

        let label = self.next_available_label;
        self.next_available_label = Label(self.next_available_label.0 + 1);
        label
    }

    /// Get the next available SSA name for the attached function.
    pub fn next_available_name(&mut self) -> Name {
        let name = self.next_available_name;
        self.next_available_name = Name(self.next_available_name.0 + 1);
        name
    }

    /// Create a new attached function overlaying the given target function.
    pub fn new(target: Arc<Function>) -> Self {
        assert!(
            target.body.keys().all(|x| *x < AttachedFunction::END_LABEL),
            "Function contains reserved labels for attached function."
        );
        let derive_dest_map = target
            .derive_dest_map()
            .iter()
            .map(|(name, instr_ref)| {
                (
                    *name,
                    (
                        *instr_ref,
                        u16::MAX, // Initialize counter to max value (cannot be removed)
                    ),
                )
            })
            .collect();

        let mut index_dedup_instr: BTreeMap<u64, SmallVec<InstructionRef, 1>> = BTreeMap::new();
        for (instr, instr_ref) in target.iter() {
            if instr.is_simple() {
                // This clone is suboptimal as the cloned instruction will be discarded
                // right after computing the hash. However, this is necessary because of
                // borrow checking and whatnot.
                let mut instr_clone = instr.clone();
                let hash = Self::compute_hash(&mut instr_clone, instr_ref.block);
                index_dedup_instr.entry(hash).or_default().push(instr_ref);
            }
        }

        Self {
            next_available_label: target.next_available_label(),
            next_available_name: target.next_available_name(),
            index_dedup_instr,
            derive_dest_map,
            target,
            overlay: BTreeMap::new(),
            begin_assert: SlotMap::new(),
            end_assert: SlotMap::new(),
        }
    }

    /// Retrieve instruction from a [`InstructionRef`].
    ///
    /// Returns [`None`] if the block or instruction index is invalid.
    pub fn get(&self, reference: InstructionRef) -> Option<&HyInstr> {
        match reference.block {
            Self::BEGIN_LABEL => {
                debug_assert!(reference.reserved != 0x0);
                self.begin_assert
                    .get(DefaultKey::from(KeyData::from_ffi(reference.reserved)))
            }
            Self::END_LABEL => {
                debug_assert!(reference.reserved != 0x0);
                self.end_assert
                    .get(DefaultKey::from(KeyData::from_ffi(reference.reserved)))
            }
            _ => {
                // Check the reserved field of the reference
                if reference.reserved == 0x0 {
                    self.target.get(reference)
                } else {
                    self.overlay.get(&reference.block).and_then(|instructions| {
                        let key = DefaultKey::from(KeyData::from_ffi(reference.reserved));
                        instructions.get(key)
                    })
                }
            }
        }
    }

    /// Find the instruction reference for a given destination name.
    pub fn find_by_dest(&self, name: &Name) -> Option<InstructionRef> {
        self.derive_dest_map
            .get(name)
            .map(|(instr_ref, _)| *instr_ref)
    }

    /// Add a new instruction to the overlay at the specified label.
    ///
    /// Returns the destination name **that may change** due to optimizations.
    /// Indeed we only push the instruction if there is no existing instruction
    /// that does the same thing.
    pub fn push(&mut self, label: Label, mut instr: HyInstr) -> (Option<Name>, InstructionRef) {
        let destination = instr.destination();
        let is_simple_instr = instr.is_simple();
        assert!(
            destination.is_none() || !self.derive_dest_map.contains_key(&destination.unwrap()),
            "Cannot add instruction with duplicate derivation destination."
        );

        // Check for existing identical instruction (ignoring destination)
        let mut hash = 0;
        if is_simple_instr {
            hash = Self::compute_hash(&mut instr, label);

            // Attempt to find existing identical instruction
            if let Some(existing_refs) = self.index_dedup_instr.get(&hash) {
                for existing_ref in existing_refs.iter() {
                    if let Some(existing_instr) = self.get(*existing_ref) {
                        // Set the destination to match for comparison (we want to ignore it)
                        if let Some(dest_name) = existing_instr.destination() {
                            instr.set_destination(dest_name);
                        }

                        // Attempt to compare instructions
                        if *existing_instr == instr {
                            // Found existing identical instruction
                            return (existing_instr.destination(), *existing_ref);
                        }
                    }
                }
            }

            // Restore original destination if not found
            if let Some(dest_name) = destination {
                instr.set_destination(dest_name);
            }
        }

        // Increase counter for instructions associated with a derivation destination
        for op in instr.operands().filter_map(Operand::try_as_reg_ref) {
            if let Some((_, counter)) = self.derive_dest_map.get_mut(op) {
                *counter = counter.saturating_add(1);
            }
        }

        let instruction_ref = match label {
            Self::BEGIN_LABEL => {
                let key = self.begin_assert.insert(instr);
                assert!(
                    key.data().as_ffi() != 0x0,
                    "Reserved field in InstructionRef cannot be zero."
                );

                InstructionRef {
                    block: Self::BEGIN_LABEL,
                    index: 0,
                    reserved: key.data().as_ffi(),
                }
            }
            Self::END_LABEL => {
                let key = self.end_assert.insert(instr);
                assert!(
                    key.data().as_ffi() != 0x0,
                    "Reserved field in InstructionRef cannot be zero."
                );

                InstructionRef {
                    block: Self::END_LABEL,
                    index: 0,
                    reserved: key.data().as_ffi(),
                }
            }
            _ => {
                assert!(
                    self.target.body.contains_key(&label),
                    "Cannot add instruction to non-existent label in target function."
                );
                let entry = self.overlay.entry(label).or_default();
                let key = entry.insert(instr);
                debug_assert!(
                    key.data().as_ffi() != 0x0,
                    "Reserved field in InstructionRef cannot be zero."
                );

                InstructionRef {
                    block: label,
                    index: 0, // Index is unused for overlay instructions
                    reserved: key.data().as_ffi(),
                }
            }
        };

        // Update the derive_dest_map (map from destination name to instruction ref and ref counter)
        if let Some(dest_name) = destination {
            self.derive_dest_map.insert(dest_name, (instruction_ref, 0));
        }

        // Update the index_of_map (for deduplication)
        if is_simple_instr {
            self.index_dedup_instr
                .entry(hash)
                .or_default()
                .push(instruction_ref);
        }

        (destination, instruction_ref)
    }

    /// Pop the instruction at the specified InstructionRef. Crashes if the ref is invalid or
    /// if the instruction has associated derivation destinations.
    pub fn pop(&mut self, reference: InstructionRef) -> HyInstr {
        assert!(
            reference.reserved != 0x0,
            "Cannot remove instruction from target function."
        );

        let instructions = match reference.block {
            Self::BEGIN_LABEL => &mut self.begin_assert,
            Self::END_LABEL => &mut self.end_assert,
            _ => self
                .overlay
                .get_mut(&reference.block)
                .expect("Cannot remove instruction from non-existent label in overlay."),
        };

        // Check whether the counter in the derive_dest_map is zero
        let key = DefaultKey::from(KeyData::from_ffi(reference.reserved));
        let instr = instructions
            .get(key)
            .expect("Cannot remove non-existent instruction from overlay.");
        if let Some(dest_name) = instr.destination()
            && let Some((_, counter)) = self.derive_dest_map.get(&dest_name)
        {
            assert!(
                *counter == 0,
                "Cannot remove instruction with associated derivation destinations."
            );
        }

        // Decrease counter for instructions associated with a derivation destination
        for op in instr.operands().filter_map(Operand::try_as_reg_ref) {
            if let Some((_, counter)) = self.derive_dest_map.get_mut(op)
                && *counter != u16::MAX
            {
                *counter -= 1;
            }
        }

        // Remove the instruction
        instructions
            .remove(key)
            .expect("Cannot remove non-existent instruction from overlay.")
    }
}

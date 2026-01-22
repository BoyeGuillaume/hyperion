use std::{collections::BTreeMap, sync::Arc};

use crate::modules::{
    Function, InstructionRef,
    instructions::HyInstr,
    operand::{Label, Name},
};

/// Represents a theorem derivation proof overlaying a target function.
///
/// This structure allows for the modification and extension of an existing function
/// by overlaying additional instructions and assertions. It maintains its own
/// labeling and naming scheme to avoid conflicts with the target function.
///
#[derive(Debug, Clone)]
pub struct TheoremDerivationProof {
    next_available_label: Label,
    next_available_name: Name,

    pub target: Arc<Function>,
    pub overlay: BTreeMap<Label, Vec<HyInstr>>,
    pub begin_assert: Option<Vec<HyInstr>>,
    pub end_assert: Option<Vec<HyInstr>>,
}

impl TheoremDerivationProof {
    pub const BEGIN_LABEL: Label = Label(u32::MAX - 1);
    pub const END_LABEL: Label = Label(u32::MAX - 2);

    /// Get the next available label for the theorem derivation proof.
    pub fn next_available_label(&mut self) -> Label {
        assert!(
            self.next_available_label < Self::END_LABEL,
            "Exceeded maximum number of labels available for theorem derivation proof."
        );

        let label = self.next_available_label;
        self.next_available_label = Label(self.next_available_label.0 + 1);
        label
    }

    /// Get the next available SSA name for the theorem derivation proof.
    pub fn next_available_name(&mut self) -> Name {
        let name = self.next_available_name;
        self.next_available_name = Name(self.next_available_name.0 + 1);
        name
    }

    /// Create a new theorem derivation proof overlaying the given target function.
    pub fn new(target: Arc<Function>) -> Self {
        assert!(
            target
                .body
                .keys()
                .all(|x| *x < TheoremDerivationProof::END_LABEL),
            "Function contains reserved labels for theorem derivation proof."
        );

        Self {
            next_available_label: target.next_available_label(),
            next_available_name: target.next_available_name(),
            target,
            overlay: BTreeMap::new(),
            begin_assert: None,
            end_assert: None,
        }
    }

    /// Retrieve instruction from a [`InstructionRef`].
    ///
    /// Returns [`None`] if the block or instruction index is invalid.
    pub fn get(&self, reference: InstructionRef) -> Option<&HyInstr> {
        match reference.block {
            Self::BEGIN_LABEL => self.begin_assert.as_ref()?.get(reference.index),
            Self::END_LABEL => self.end_assert.as_ref()?.get(reference.index),
            _ => {
                if let Some(instructions) = self.overlay.get(&reference.block) {
                    if let Some(instr) = instructions.get(reference.index) {
                        return Some(instr);
                    }
                }

                self.target.get(reference)
            }
        }
    }
}

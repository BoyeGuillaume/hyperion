use std::{collections::BTreeMap, sync::Arc};

use crate::modules::{Function, instructions::HyInstr, operand::Label};

pub struct TheoremDerivationProof {
    pub target: Arc<Function>,
    pub overlay: BTreeMap<Label, Vec<HyInstr>>,
    pub begin_assert: Option<Vec<HyInstr>>,
    pub end_assert: Option<Vec<HyInstr>>,
}

impl TheoremDerivationProof {
    pub const BEGIN_LABEL: Label = Label(u32::MAX - 1);
    pub const END_LABEL: Label = Label(u32::MAX - 2);

    pub fn new(target: Arc<Function>) -> Self {
        assert!(
            target
                .body
                .keys()
                .all(|x| x.0 < TheoremDerivationProof::END_LABEL.0),
            "Function contains reserved labels for theorem derivation proof."
        );

        Self {
            target,
            overlay: BTreeMap::new(),
            begin_assert: None,
            end_assert: None,
        }
    }
}

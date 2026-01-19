use std::collections::BTreeMap;
use uuid::Uuid;

use crate::modules::{instructions::HyInstr, operand::Label};

pub struct TheoremDerivationProof {
    pub target_uuid: Uuid,
    pub overlay: BTreeMap<Label, Vec<HyInstr>>,
    pub begin_assert: Option<Vec<HyInstr>>,
    pub end_assert: Option<Vec<HyInstr>>,
}

impl TheoremDerivationProof {
    pub fn new(target_uuid: Uuid) -> Self {
        Self {
            target_uuid,
            overlay: BTreeMap::new(),
            begin_assert: None,
            end_assert: None,
        }
    }
}

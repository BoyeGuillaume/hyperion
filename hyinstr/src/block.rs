use crate::{instr::HyInstr, terminator::HyTerminator};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BasicBlock {
    pub instructions: Vec<HyInstr>,
    pub terminator: HyTerminator,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Function {
    pub blocks: Vec<BasicBlock>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Module {
    pub functions: Vec<Function>,
}

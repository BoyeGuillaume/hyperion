use std::collections::BTreeMap;

use crate::{instr::HyInstr, name::Name, terminator::HyTerminator};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// A basic block containing a sequence of instructions and a terminator.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct BasicBlock {
    pub name: Name,
    pub instructions: Vec<HyInstr>,
    pub terminator: HyTerminator,
}

/// Visibility of functions and global variables.
///
/// This closely mirrors LLVM's visibility styles.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Visibility {
    Hidden,
    Protected,
}

/// A function consisting of multiple basic blocks.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Function {
    pub identifier: Name,
    pub visibility: Visibility,
    pub blocks: BTreeMap<Name, BasicBlock>,
}

/// A module containing multiple functions.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Module {
    pub functions: BTreeMap<Name, Function>,
}

//! Instruction IR modules
//!
//! This module groups all instruction kinds exposed by the Hy instruction
//! IR. Each instruction is represented as a small data structure with public
//! fields, making it easy to construct and inspect. Submodules contain
//! families of operations:
//!
//! - `int`: integer arithmetic, comparisons, shifts and bitwise ops
//! - `fp`: floating‑point arithmetic and comparisons
//! - `mem`: memory loads and stores with optional atomic semantics
//! - `operand`: shared operand and SSA name types
//!
//! You typically manipulate instructions via the `HyInstr` enum which is a
//! tagged union of all concrete instruction forms.
use crate::modules::operand::{Name, Operand};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use strum::{EnumDiscriminants, EnumIs, EnumTryAs};

pub mod fp;
pub mod int;
pub mod mem;
pub mod operand;

/// Common interface implemented by every instruction node.
///
/// This trait provides lightweight, zero‑allocation iteration over an
/// instruction's input operands and exposes its optional destination SSA
/// name when present.
pub trait Instruction {
    /// Iterate over all input operands for this instruction.
    fn operands(&self) -> impl Iterator<Item = &Operand>;

    /// Return the destination SSA name if the instruction produces a result.
    fn destination(&self) -> Option<Name> {
        None
    }

    /// Convenience iterator over referenced SSA names (i.e., register
    /// operands). Immediates and labels are ignored.
    fn name_dependencies(&self) -> impl Iterator<Item = Name> {
        self.operands().filter_map(|op| match op {
            Operand::Reg(reg) => Some(*reg),
            _ => None,
        })
    }
}

/// Discriminated union covering all public instruction kinds.
///
/// Use this enum to store heterogeneous instruction streams and to pattern‑match
/// on specific operations. The generated `HyInstrKind` discriminant (via
/// `strum`) can be helpful for fast classification.
#[derive(
    Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, EnumIs, EnumTryAs, EnumDiscriminants,
)]
#[strum_discriminants(name(HyInstrKind))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum HyInstr {
    // Integer instructions
    IAdd(int::IAdd),
    ISub(int::ISub),
    IMul(int::IMul),
    IDiv(int::IDiv),
    IRem(int::IRem),
    ICmp(int::ICmp),
    ISht(int::ISht),
    INeg(int::INeg),

    // Bitwise instructions
    IAnd(int::IAnd),
    IOr(int::IOr),
    IXor(int::IXor),
    INot(int::INot),

    // Floating-point instructions
    FAdd(fp::FAdd),
    FSub(fp::FSub),
    FMul(fp::FMul),
    FDiv(fp::FDiv),
    FRem(fp::FRem),
    FCmp(fp::FCmp),
    FNeg(fp::FNeg),

    // Memory instructions
    MLoad(mem::MLoad),
    MStore(mem::MStore),
}

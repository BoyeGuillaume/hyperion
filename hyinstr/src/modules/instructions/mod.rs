use auto_enums::auto_enum;
use bitflags::bitflags;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use strum::{EnumDiscriminants, EnumIs, EnumIter, EnumTryAs, IntoEnumIterator};

use crate::{
    modules::{Operand, operand::Name},
    types::Typeref,
};

pub mod fp;
pub mod int;
pub mod mem;
pub mod meta;
pub mod misc;

bitflags! {
    /// Flags providing additional information about instructions, this can
    /// be whether an instruction is a meta-instruction, whether it has side-effects, etc.
    #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    pub struct InstructionFlags: u32 {
        /// Instruction is a meta-instruction (e.g., assertions, assumptions, ...)
        ///
        /// Those instructions are not meant to appear in executable code. They are
        /// used for verification, analysis, or optimization purposes.
        const META = 1 << 0;

        /// Instruction defined are simple which is a weaker form of having no side-effects.
        ///
        /// A "simple" instruction is one that does not have side-effects except for potential
        /// trap due to overflow or invalid operations (e.g., division by zero).
        ///
        /// The intuition behind is that it could be freely duplicated (e.g., inlining) and
        /// that removing duplicated simple instructions would not change the program semantics.
        /// It cannot be fully removed as trapping behavior must be preserved.
        ///
        /// 1. Memory instructions are *never* "simple" even if technically non-volatile loads
        ///    could be considered as such.
        /// 2. meta assert/assume/prob are considered simple as they can be duplicated without
        ///    changing semantics.
        /// 3. Invoke instructions are not simple as they may have side-effects.
        /// 4. Phi instructions are considered simple as they are just SSA value selectors.
        /// 5. All arithmetic and logical instructions are considered simple.
        /// 6. Select instructions are considered simple as they are just SSA value selectors.
        const SIMPLE = 1 << 1;

        /// This instruction is an arithmetic operation
        ///
        /// Used to group both integer and floating-point arithmetic instructions.
        const ARITHMETIC = 1 << 6;

        /// This instruction is a integer arithmetic operation
        ///
        /// This includes all integer arithmetic and integer comparison instructions (e.g., iadd, isub, imul, idiv, icmp)
        const ARITHMETIC_INT = Self::ARITHMETIC.bits() | (1 << 7);

        /// This instruction is a floating-point arithmetic operation
        ///
        /// This includes all FP arithmetic and FP comparison instructions (e.g., fadd, fsub, fmul, fdiv, fcmp)
        const ARITHMETIC_FP = Self::ARITHMETIC.bits() | (1 << 8);

        /// This flag is reserved for invoke instructions as `invoke` have behavior
        /// depending on function external to the current function/module. As such
        /// it should be understand as "maybe has side-effects".
        const INVOKE = 1 << 9;

        /// This instruction is affecting or accessing memory state. This
        /// regroups loads, stores
        const MEMORY = 1 << 10;

        /// Control flow instruction (e.g., function calls, branches) also for every terminator instruction.
        const CONTROL_FLOW = 1 << 11;

        /// Terminator instruction (ending a basic block)
        const TERMINATOR = 1 << 12 | Self::CONTROL_FLOW.bits();
    }
}

#[cfg(feature = "borsh")]
impl borsh::BorshSerialize for InstructionFlags {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        borsh::BorshSerialize::serialize(&self.bits(), writer)
    }
}

#[cfg(feature = "borsh")]
impl borsh::BorshDeserialize for InstructionFlags {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let bits = borsh::BorshDeserialize::deserialize_reader(reader)?;
        Ok(InstructionFlags::from_bits_truncate(bits))
    }
}

/// Common interface implemented by every instruction node.
///
/// This trait provides lightweight, zero‑allocation iteration over an
/// instruction's input operands and exposes its optional destination SSA
/// name when present.
pub trait Instruction {
    /// Get the instruction flags for this instruction.
    fn flags(&self) -> InstructionFlags;

    /// Returns true if this instruction is a meta-instruction.
    #[inline]
    fn is_meta_instruction(&self) -> bool {
        self.flags().contains(InstructionFlags::META)
    }

    /// Returns true if this instruction is "simple", see [`InstructionFlags::SIMPLE`].
    #[inline]
    fn is_simple(&self) -> bool {
        self.flags().contains(InstructionFlags::SIMPLE)
    }

    /// Iterate over all input operands for this instruction.
    fn operands(&self) -> impl Iterator<Item = &Operand>;

    /// Return the destination SSA name if the instruction produces a result.
    fn destination(&self) -> Option<Name> {
        None
    }

    /// Type of the destination SSA name if the instruction produces a result.
    fn destination_type(&self) -> Option<Typeref> {
        None
    }

    /// Any types referenced by this instruction.
    fn referenced_types(&self) -> impl Iterator<Item = Typeref>;

    /// Any types referenced by this instruction mutably.
    fn referenced_types_mut(&mut self) -> impl Iterator<Item = &mut Typeref>;

    /// Update the destination SSA name for this instruction. No-op if the
    /// instruction does not produce a result.
    fn set_destination(&mut self, _name: Name) {}

    /// Mutably iterate over all input operands for this instruction.
    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand>;

    /// Convenience iterator over referenced SSA names (i.e., register
    /// operands). Immediates and labels are ignored.
    fn dependencies(&self) -> impl Iterator<Item = Name> {
        self.operands().filter_map(|op| match op {
            Operand::Reg(reg) => Some(*reg),
            _ => None,
        })
    }

    fn dependencies_mut(&mut self) -> impl Iterator<Item = &mut Name> {
        self.operands_mut().filter_map(|op| match op {
            Operand::Reg(reg) => Some(reg),
            _ => None,
        })
    }

    /// Remap operands according to a mapping
    fn remap_operands(&mut self, mapping: impl Fn(Name) -> Option<Name>) {
        for operand in self.operands_mut() {
            if let Operand::Reg(name) = operand
                && let Some(new_name) = mapping(*name)
            {
                *name = new_name;
            }
        }
    }

    /// Remap types according to a mapping
    fn remap_types(&mut self, mapping: impl Fn(Typeref) -> Option<Typeref>) {
        for ty in self.referenced_types_mut() {
            if let Some(new_ty) = mapping(*ty) {
                *ty = new_ty;
            }
        }
    }
}

/// Discriminated union covering all public instruction kinds.
///
/// Use this enum to store heterogeneous instruction streams and to pattern‑match
/// on specific operations. The generated `HyInstrKind` discriminant (via
/// `strum`) can be helpful for fast classification.
#[derive(Debug, Clone, Hash, PartialEq, Eq, EnumIs, EnumTryAs, EnumDiscriminants)]
#[strum_discriminants(name(HyInstrOp), derive(EnumIter))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
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
    IImplies(int::IImplies),
    IEquiv(int::IEquiv),

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
    MAlloca(mem::MAlloca),
    MGetElementPtr(mem::MGetElementPtr),

    // Misc instructions
    Invoke(misc::Invoke),
    Phi(misc::Phi),
    Select(misc::Select),
    Cast(misc::Cast),
    InsertValue(misc::InsertValue),
    ExtractValue(misc::ExtractValue),

    // Meta instructions
    MetaAssert(meta::MetaAssert),
    MetaAssume(meta::MetaAssume),
    MetaIsDef(meta::MetaIsDef),
    MetaProb(meta::MetaProb),
    MetaAnalysisStat(meta::MetaAnalysisStat),
    MetaForall(meta::MetaForall),
}

impl HyInstrOp {
    /// Return the canonical mnemonic used when printing this instruction.
    pub fn opname(&self) -> &'static str {
        match self {
            HyInstrOp::IAdd => "iadd",
            HyInstrOp::ISub => "isub",
            HyInstrOp::IMul => "imul",
            HyInstrOp::IDiv => "idiv",
            HyInstrOp::IRem => "irem",
            HyInstrOp::ICmp => "icmp",
            HyInstrOp::ISht => "isht",
            HyInstrOp::INeg => "ineg",

            HyInstrOp::IAnd => "and",
            HyInstrOp::IOr => "or",
            HyInstrOp::IXor => "xor",
            HyInstrOp::INot => "not",
            HyInstrOp::IImplies => "implies",
            HyInstrOp::IEquiv => "equiv",

            HyInstrOp::FAdd => "fadd",
            HyInstrOp::FSub => "fsub",
            HyInstrOp::FMul => "fmul",
            HyInstrOp::FDiv => "fdiv",
            HyInstrOp::FRem => "frem",
            HyInstrOp::FCmp => "fcmp",
            HyInstrOp::FNeg => "fneg",

            HyInstrOp::MLoad => "load",
            HyInstrOp::MStore => "store",
            HyInstrOp::MAlloca => "alloca",
            HyInstrOp::MGetElementPtr => "getelementptr",

            HyInstrOp::Invoke => "invoke",
            HyInstrOp::Phi => "phi",
            HyInstrOp::Select => "select",
            HyInstrOp::Cast => "cast",
            HyInstrOp::InsertValue => "insertvalue",
            HyInstrOp::ExtractValue => "extractvalue",

            HyInstrOp::MetaAssert => "!assert",
            HyInstrOp::MetaAssume => "!assume",
            HyInstrOp::MetaIsDef => "!isdef",
            HyInstrOp::MetaProb => "!prob",
            HyInstrOp::MetaAnalysisStat => "!analysis",
            HyInstrOp::MetaForall => "!forall",
        }
    }

    /// Return the fixed operand count if the instruction has one.
    pub fn arity(&self) -> Option<usize> {
        match self {
            HyInstrOp::INeg | HyInstrOp::INot | HyInstrOp::FNeg | HyInstrOp::Cast => Some(1),
            HyInstrOp::IAdd
            | HyInstrOp::ISub
            | HyInstrOp::IMul
            | HyInstrOp::IDiv
            | HyInstrOp::IRem
            | HyInstrOp::ICmp
            | HyInstrOp::ISht
            | HyInstrOp::IAnd
            | HyInstrOp::IOr
            | HyInstrOp::IXor
            | HyInstrOp::IImplies
            | HyInstrOp::IEquiv
            | HyInstrOp::FAdd
            | HyInstrOp::FSub
            | HyInstrOp::FMul
            | HyInstrOp::FDiv
            | HyInstrOp::FRem
            | HyInstrOp::FCmp => Some(2), // binary ops
            HyInstrOp::MLoad => Some(1),       // ptr
            HyInstrOp::MStore => Some(2),      // ptr + value
            HyInstrOp::MAlloca => Some(1),     // allocation size
            HyInstrOp::MGetElementPtr => None, // variable arity (at least 2 - base ptr + one index)
            HyInstrOp::Invoke => None,         // variable arity (at least 1 - function ptr)
            HyInstrOp::Phi => None,            // variable arity (at least 1)
            HyInstrOp::Select => Some(3),      // cond + val_true + val_false
            HyInstrOp::InsertValue => None,    // aggregate + value + indices
            HyInstrOp::ExtractValue => None,   // aggregate + indices
            HyInstrOp::MetaAssert | HyInstrOp::MetaAssume => Some(1), // condition
            HyInstrOp::MetaIsDef => Some(1),
            HyInstrOp::MetaProb => None, // variable arity depending on variant
            HyInstrOp::MetaAnalysisStat => None, // variable arity depending on analysis op
            HyInstrOp::MetaForall => Some(0),
        }
    }

    /// Return true when the instruction carries an additional variant field.
    pub fn has_variant(&self) -> bool {
        matches!(
            self,
            HyInstrOp::ICmp
                | HyInstrOp::FCmp
                | HyInstrOp::Cast
                | HyInstrOp::ISht
                | HyInstrOp::MetaProb
                | HyInstrOp::MetaAnalysisStat
                | HyInstrOp::IAdd
                | HyInstrOp::ISub
                | HyInstrOp::IMul
                | HyInstrOp::IDiv
                | HyInstrOp::IRem
        )
    }
}

impl std::str::FromStr for HyInstrOp {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        HyInstrOp::iter().find(|op| op.opname() == s).ok_or(())
    }
}

impl HyInstr {
    /// Return the discriminant for this instruction value.
    pub fn op(&self) -> HyInstrOp {
        self.into()
    }
}

macro_rules! define_instr_any_instr {
    (
        $($variant:ident),* $(,)?
    ) => {
        impl Instruction for HyInstr {
            fn flags(&self) -> InstructionFlags {
                match self {
                    $(
                        HyInstr::$variant(instr) => instr.flags(),
                    )*
                }
            }

            #[auto_enum(Iterator)]
            fn operands(&self) -> impl Iterator<Item = &Operand> {
                match self {
                    $(
                        HyInstr::$variant(instr) => instr.operands(),
                    )*
                }
            }

            fn destination(&self) -> Option<super::operand::Name> {
                match self {
                    $(
                        HyInstr::$variant(instr) => instr.destination(),
                    )*
                }
            }

            #[auto_enum(Iterator)]
            fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
                match self {
                    $(
                        HyInstr::$variant(instr) => instr.operands_mut(),
                    )*
                }
            }

            fn set_destination(&mut self, name: super::operand::Name) {
                match self {
                    $(
                        HyInstr::$variant(instr) => instr.set_destination(name),
                    )*
                }
            }

            #[auto_enum(Iterator)]
            fn referenced_types(&self) -> impl Iterator<Item = crate::types::Typeref> {
                match self {
                    $(
                        HyInstr::$variant(instr) => instr.referenced_types(),
                    )*
                }
            }

            #[auto_enum(Iterator)]
            fn referenced_types_mut(&mut self) -> impl Iterator<Item = &mut crate::types::Typeref> {
                match self {
                    $(
                        HyInstr::$variant(instr) => instr.referenced_types_mut(),
                    )*
                }
            }

            fn destination_type(&self) -> Option<crate::types::Typeref> {
                match self {
                    $(
                        HyInstr::$variant(instr) => instr.destination_type(),
                    )*
                }
            }
        }
    };
}

define_instr_any_instr! {
    IAdd,
    ISub,
    IMul,
    IDiv,
    IRem,
    ICmp,
    ISht,
    INeg,
    IAnd,
    IOr,
    IXor,
    INot,
    IImplies,
    IEquiv,
    FAdd,
    FSub,
    FMul,
    FDiv,
    FRem,
    FCmp,
    FNeg,
    MLoad,
    MStore,
    MAlloca,
    MGetElementPtr,
    Invoke,
    Phi,
    Select,
    Cast,
    InsertValue,
    ExtractValue,
    MetaAssert,
    MetaAssume,
    MetaIsDef,
    MetaProb,
    MetaAnalysisStat,
    MetaForall,
}

macro_rules! define_hyinstr_from {
    ($typ:ty, $variant:ident) => {
        impl From<$typ> for HyInstr {
            fn from(inst: $typ) -> Self {
                HyInstr::$variant(inst)
            }
        }
    };
}

define_hyinstr_from!(int::IAdd, IAdd);
define_hyinstr_from!(int::ISub, ISub);
define_hyinstr_from!(int::IMul, IMul);
define_hyinstr_from!(int::IDiv, IDiv);
define_hyinstr_from!(int::IRem, IRem);
define_hyinstr_from!(int::ICmp, ICmp);
define_hyinstr_from!(int::ISht, ISht);
define_hyinstr_from!(int::INeg, INeg);
define_hyinstr_from!(int::IAnd, IAnd);
define_hyinstr_from!(int::IOr, IOr);
define_hyinstr_from!(int::IXor, IXor);
define_hyinstr_from!(int::INot, INot);
define_hyinstr_from!(int::IImplies, IImplies);
define_hyinstr_from!(int::IEquiv, IEquiv);

define_hyinstr_from!(fp::FAdd, FAdd);
define_hyinstr_from!(fp::FSub, FSub);
define_hyinstr_from!(fp::FMul, FMul);
define_hyinstr_from!(fp::FDiv, FDiv);
define_hyinstr_from!(fp::FRem, FRem);
define_hyinstr_from!(fp::FCmp, FCmp);
define_hyinstr_from!(fp::FNeg, FNeg);

define_hyinstr_from!(mem::MLoad, MLoad);
define_hyinstr_from!(mem::MStore, MStore);
define_hyinstr_from!(mem::MAlloca, MAlloca);
define_hyinstr_from!(mem::MGetElementPtr, MGetElementPtr);

define_hyinstr_from!(misc::Invoke, Invoke);
define_hyinstr_from!(misc::Phi, Phi);
define_hyinstr_from!(misc::Select, Select);
define_hyinstr_from!(misc::Cast, Cast);
define_hyinstr_from!(misc::InsertValue, InsertValue);
define_hyinstr_from!(misc::ExtractValue, ExtractValue);

define_hyinstr_from!(meta::MetaAssert, MetaAssert);
define_hyinstr_from!(meta::MetaAssume, MetaAssume);
define_hyinstr_from!(meta::MetaIsDef, MetaIsDef);
define_hyinstr_from!(meta::MetaProb, MetaProb);
define_hyinstr_from!(meta::MetaAnalysisStat, MetaAnalysisStat);
define_hyinstr_from!(meta::MetaForall, MetaForall);

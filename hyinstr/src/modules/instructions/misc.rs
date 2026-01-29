//! Miscellaneous instructions that do not fit in arithmetic or memory groups.
//!
//! These include function calls, phi nodes, select operations, and casts.
use crate::{
    modules::{
        CallingConvention,
        instructions::{Instruction, InstructionFlags},
        operand::{Label, Name, Operand},
    },
    types::Typeref,
};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoEnumIterator};

/// Function call instruction
///
/// In hyperion, function cannot raise exceptions; thus, it will always jump to
/// the specified `exit_label` after the call completes. In case of errors, either use
/// a return code or never return from the function (e.g., abort).
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct Invoke {
    /// Should be a reference to a function pointer (either internal or external). We
    /// describe it as an `Operand` to allow dynamic function calls to achieve virtualization
    /// or function pointer tables.
    pub function: Operand,

    /// The argument operands to pass to the function.
    pub args: Vec<Operand>,

    /// The destination SSA name for the return value, if any.
    pub dest: Option<Name>,

    /// The return type of the function being called. `None` for `void` functions.
    pub ty: Option<Typeref>,

    /// This should only be `Some` for calls to external functions (i.e., not
    /// defined within the current module)
    pub cconv: Option<CallingConvention>,
}

impl Instruction for Invoke {
    fn flags(&self) -> InstructionFlags {
        InstructionFlags::INVOKE | InstructionFlags::CONTROL_FLOW
    }

    fn operands(&self) -> impl Iterator<Item = &Operand> {
        std::iter::once(&self.function).chain(self.args.iter())
    }

    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        std::iter::once(&mut self.function).chain(self.args.iter_mut())
    }

    fn destination(&self) -> Option<Name> {
        self.dest
    }

    fn set_destination(&mut self, name: Name) {
        // Cannot change a void return to a non-void return
        if self.dest.is_some() {
            self.dest = Some(name);
        }
    }

    fn referenced_types(&self) -> impl Iterator<Item = Typeref> {
        self.ty.into_iter()
    }

    fn referenced_types_mut(&mut self) -> impl Iterator<Item = &mut Typeref> {
        self.ty.iter_mut()
    }

    fn destination_type(&self) -> Option<Typeref> {
        self.ty
    }
}

/// Phi instruction
///
/// This instruction selects a value based on control flow. It is used to merge
/// values coming from different basic blocks. It should always be placed at the
/// beginning of a basic block.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct Phi {
    /// The destination SSA name for the result of the phi instruction.
    pub dest: Name,

    /// The type of the value being selected.
    pub ty: Typeref,

    /// The incoming values and their corresponding predecessor basic blocks.
    pub values: Vec<(Operand, Label)>, // (predecessor block label, value name)
}

impl Instruction for Phi {
    fn flags(&self) -> InstructionFlags {
        InstructionFlags::SIMPLE
    }

    fn operands(&self) -> impl Iterator<Item = &Operand> {
        self.values.iter().map(|(op, _)| op)
    }

    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        self.values.iter_mut().map(|(op, _)| op)
    }

    fn destination(&self) -> Option<Name> {
        Some(self.dest)
    }

    fn set_destination(&mut self, name: Name) {
        self.dest = name;
    }

    fn referenced_types(&self) -> impl Iterator<Item = Typeref> {
        std::iter::once(self.ty)
    }

    fn referenced_types_mut(&mut self) -> impl Iterator<Item = &mut Typeref> {
        std::iter::once(&mut self.ty)
    }

    fn destination_type(&self) -> Option<Typeref> {
        Some(self.ty)
    }
}

/// Select instruction
///
/// This instruction selects one of two values based on a condition.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct Select {
    /// The destination SSA name for the result of the select instruction.
    pub dest: Name,
    /// The condition operand. Should evaluate to a boolean value.
    pub condition: Operand,
    /// The operand to select if the condition is true.
    pub true_value: Operand,
    /// The operand to select if the condition is false.
    pub false_value: Operand,
    /// The type of the values being selected.
    pub ty: Typeref,
}

impl Instruction for Select {
    fn flags(&self) -> InstructionFlags {
        InstructionFlags::SIMPLE
    }

    fn operands(&self) -> impl Iterator<Item = &Operand> {
        std::iter::once(&self.condition)
            .chain(std::iter::once(&self.true_value))
            .chain(std::iter::once(&self.false_value))
    }

    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        std::iter::once(&mut self.condition)
            .chain(std::iter::once(&mut self.true_value))
            .chain(std::iter::once(&mut self.false_value))
    }

    fn destination(&self) -> Option<Name> {
        Some(self.dest)
    }

    fn set_destination(&mut self, name: Name) {
        self.dest = name;
    }

    fn referenced_types(&self) -> impl Iterator<Item = Typeref> {
        std::iter::once(self.ty)
    }

    fn referenced_types_mut(&mut self) -> impl Iterator<Item = &mut Typeref> {
        std::iter::once(&mut self.ty)
    }

    fn destination_type(&self) -> Option<Typeref> {
        Some(self.ty)
    }
}

/// Cast operation enumeration
///
/// This enum defines the various casting operations supported in Hyperion. See [`Cast`] for more
/// details.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, EnumIter)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub enum CastVariant {
    /// Truncate integer
    ///
    /// Converts integer to integer of smaller width.
    Trunc,

    /// Zero extend integer
    ///
    /// Converts unsigned integer to unsigned integer of larger width. New bits are set to 0.
    ZExt,

    /// Sign extend integer
    ///
    /// Converts signed integer to signed integer of larger width. If the value is negative,
    /// the new bits are set to 1, otherwise to 0.
    SExt,

    /// Float to float truncation
    ///
    /// Converts float to float of smaller width.
    FpTrunc,

    /// Float to float extension
    ///
    /// Converts float to float of larger width.
    FpExt,

    /// Float to unsigned integer
    ///
    /// Converts float to unsigned integer, always rounding toward zero. If value cannot fit in ty2,
    /// behavior is undefined (poison value).
    FpToUI,

    /// Float to signed integer
    ///
    /// Converts float to signed integer, always rounding toward zero. If value cannot fit in ty2,
    /// behavior is undefined (poison value).
    FpToSI,
    /// Unsigned integer to float
    ///
    /// Converts unsigned integer to float.
    UIToFp,

    /// Signed integer to float
    ///
    /// Converts signed integer to float.
    SIToFp,

    /// Ptr to integer
    ///
    /// Converts a pointer to an integer type. If pointer is smaller than integer type, it
    /// is zero-extended. If larger, it is truncated.
    PtrToInt,

    /// Integer to ptr
    ///
    /// Converts an integer to a pointer type. If integer is smaller than pointer type, it
    /// is zero-extended. If larger, it is truncated.
    IntToPtr,

    /// Bitcast
    ///
    /// Converts between types of the same size without changing the bit representation.
    Bitcast,
}

impl CastVariant {
    /// Returns a string representation of the cast operation.
    pub fn to_str(&self) -> &'static str {
        match self {
            CastVariant::Trunc => "trunc",
            CastVariant::ZExt => "zext",
            CastVariant::SExt => "sext",
            CastVariant::FpTrunc => "fptrunc",
            CastVariant::FpExt => "fpext",
            CastVariant::FpToUI => "fptoui",
            CastVariant::FpToSI => "fptosi",
            CastVariant::UIToFp => "uitofp",
            CastVariant::SIToFp => "sitofp",
            CastVariant::PtrToInt => "ptrtoint",
            CastVariant::IntToPtr => "inttoptr",
            CastVariant::Bitcast => "bitcast",
        }
    }
}

impl std::str::FromStr for CastVariant {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        CastVariant::iter().find(|op| op.to_str() == s).ok_or(())
    }
}

/// Cast instruction
///
/// This instruction casts a value from one type to another using the specified cast operation.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct Cast {
    /// The destination SSA name for the casted result.
    pub dest: Name,
    /// The type to cast to.
    pub ty: Typeref,
    /// The source operand to cast.
    pub value: Operand,
    /// The cast operation to perform.
    pub variant: CastVariant,
}

impl Instruction for Cast {
    fn flags(&self) -> InstructionFlags {
        InstructionFlags::SIMPLE
    }

    fn operands(&self) -> impl Iterator<Item = &Operand> {
        std::iter::once(&self.value)
    }

    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        std::iter::once(&mut self.value)
    }

    fn destination(&self) -> Option<Name> {
        Some(self.dest)
    }

    fn set_destination(&mut self, name: Name) {
        self.dest = name;
    }

    fn referenced_types(&self) -> impl Iterator<Item = Typeref> {
        std::iter::once(self.ty)
    }

    fn referenced_types_mut(&mut self) -> impl Iterator<Item = &mut Typeref> {
        std::iter::once(&mut self.ty)
    }

    fn destination_type(&self) -> Option<Typeref> {
        Some(self.ty)
    }
}

/// Insert a value into an aggregate.
///
/// Mirrors LLVM's `insertvalue`: takes an aggregate SSA value and returns a new SSA value of the
/// same aggregate type with the element at the given index path replaced. The `indices` vector is
/// a path into potentially nested aggregates (e.g., struct-of-array: first index selects the
/// struct field, second index selects the array element). All indices must be constant integers.
/// This is purely a value transform; no memory is touched.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct InsertValue {
    /// Destination SSA name receiving the updated aggregate.
    pub dest: Name,
    /// Resulting aggregate type.
    pub ty: Typeref,
    /// Source aggregate operand.
    pub aggregate: Operand,
    /// Value to insert.
    pub value: Operand,
    /// Path of indices identifying the insertion point.
    pub indices: Vec<u32>,
}

impl Instruction for InsertValue {
    fn flags(&self) -> InstructionFlags {
        InstructionFlags::SIMPLE
    }

    fn operands(&self) -> impl Iterator<Item = &Operand> {
        std::iter::once(&self.aggregate).chain(std::iter::once(&self.value))
    }

    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        std::iter::once(&mut self.aggregate).chain(std::iter::once(&mut self.value))
    }

    fn destination(&self) -> Option<Name> {
        Some(self.dest)
    }

    fn set_destination(&mut self, name: Name) {
        self.dest = name;
    }

    fn referenced_types(&self) -> impl Iterator<Item = Typeref> {
        std::iter::once(self.ty)
    }

    fn referenced_types_mut(&mut self) -> impl Iterator<Item = &mut Typeref> {
        std::iter::once(&mut self.ty)
    }

    fn destination_type(&self) -> Option<Typeref> {
        Some(self.ty)
    }
}

/// Extract a value from an aggregate.
///
/// Mirrors LLVM's `extractvalue`: takes an aggregate SSA value and yields the element at the given
/// index path. The `indices` vector walks through nested aggregates exactly like `InsertValue`, and
/// all indices must be constant integers. This is a pure value operation (no memory access).
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct ExtractValue {
    /// Destination SSA name receiving the extracted element.
    pub dest: Name,
    /// Resulting element type.
    pub ty: Typeref,
    /// Source aggregate operand.
    pub aggregate: Operand,
    /// Path of indices identifying the extraction point.
    pub indices: Vec<u32>,
}

impl Instruction for ExtractValue {
    fn flags(&self) -> InstructionFlags {
        InstructionFlags::SIMPLE
    }

    fn operands(&self) -> impl Iterator<Item = &Operand> {
        std::iter::once(&self.aggregate)
    }

    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        std::iter::once(&mut self.aggregate)
    }

    fn destination(&self) -> Option<Name> {
        Some(self.dest)
    }

    fn set_destination(&mut self, name: Name) {
        self.dest = name;
    }

    fn referenced_types(&self) -> impl Iterator<Item = Typeref> {
        std::iter::once(self.ty)
    }

    fn referenced_types_mut(&mut self) -> impl Iterator<Item = &mut Typeref> {
        std::iter::once(&mut self.ty)
    }

    fn destination_type(&self) -> Option<Typeref> {
        Some(self.ty)
    }
}

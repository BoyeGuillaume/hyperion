//! Memory operations
//!
//! Load and store instructions with alignment, volatility, and optional
//! atomic ordering semantics compatible with common language memory models
//! (C++/Java). The exact effects of `MemoryOrdering` follow the referenced
//! specifications; only user‑visible controls are documented here.
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoEnumIterator};

use crate::{
    modules::{
        instructions::{Instruction, InstructionFlags},
        operand::{Name, Operand},
    },
    types::Typeref,
};

/// Ordering for atomic memory operations.
///
/// Certain atomic instructions take ordering parameters that determine which
/// other atomic instructions on the same address they synchronize with. These
/// semantics implement the Java or C++ memory models; if these descriptions
/// aren't precise enough, check those specs
/// (see specs references on [cppreference](https://en.cppreference.com/w/cpp/atomic/memory_order)).
/// You can also check LLVM's documentation on [Ordering](https://llvm.org/docs/LangRef.html#atomic-memory-ordering) for more details.
#[derive(Debug, Clone, Hash, PartialEq, Eq, EnumIter)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub enum MemoryOrdering {
    Unordered,
    Monotonic,
    Acq,
    Rel,
    AcqRel,
    SeqCst,
}

impl std::str::FromStr for MemoryOrdering {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        MemoryOrdering::iter()
            .find(|ord| ord.to_str() == s)
            .ok_or(())
    }
}

impl MemoryOrdering {
    /// Return the canonical mnemonic for this memory ordering.
    pub fn to_str(&self) -> &'static str {
        match self {
            MemoryOrdering::Unordered => "unordered",
            MemoryOrdering::Monotonic => "monotonic",
            MemoryOrdering::Acq => "acquire",
            MemoryOrdering::Rel => "release",
            MemoryOrdering::AcqRel => "acq_rel",
            MemoryOrdering::SeqCst => "seq_cst",
        }
    }
}

/// Load from memory into a destination SSA name.
///
/// When `volatile` is true, the operation is prevented from being removed or
/// merged by typical optimizations. If an `ordering` other than `Unordered`
/// is specified, the load is considered atomic with the given ordering.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct MLoad {
    /// Destination SSA name receiving the loaded value.
    pub dest: Name,
    /// Type of the loaded value.
    pub ty: Typeref,
    /// Pointer operand describing the source address.
    pub addr: Operand,
    /// Optional byte alignment hint for the access.
    pub alignement: Option<u32>,

    /// A notable distinction with LLVM's memory model is that Hyperion does
    /// not allow syncscope('singlethread') operations; all atomic operations
    /// are assumed to be cross‑thread unless the access is non‑atomic.
    ///
    /// When present, the load is treated as atomic with the supplied ordering.
    pub ordering: Option<MemoryOrdering>,
    /// When true, the load is considered volatile.
    pub volatile: bool,
}

impl Instruction for MLoad {
    fn flags(&self) -> InstructionFlags {
        InstructionFlags::MEMORY
    }

    fn operands(&self) -> impl Iterator<Item = &Operand> {
        std::iter::once(&self.addr)
    }

    fn destination(&self) -> Option<Name> {
        Some(self.dest)
    }

    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        std::iter::once(&mut self.addr)
    }

    fn set_destination(&mut self, name: Name) {
        self.dest = name;
    }

    fn destination_type(&self) -> Option<Typeref> {
        Some(self.ty)
    }

    fn referenced_types(&self) -> impl Iterator<Item = Typeref> {
        std::iter::once(self.ty)
    }

    fn referenced_types_mut(&mut self) -> impl Iterator<Item = &mut Typeref> {
        std::iter::once(&mut self.ty)
    }
}

/// Store a value to memory.
///
/// When `volatile` is true, the operation is prevented from being removed or
/// merged by typical optimizations. If an `ordering` other than `Unordered`
/// is specified, the store is considered atomic with the given ordering.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct MStore {
    /// Pointer operand describing the destination address.
    pub addr: Operand,
    /// Value to store.
    pub value: Operand,
    /// Optional byte alignment hint for the access.
    pub alignement: Option<u32>,

    /// A notable distinction with LLVM's memory model is that Hyperion does
    /// not allow syncscope('singlethread') operations; all atomic operations
    /// are assumed to be cross‑thread unless the access is non‑atomic.
    ///
    /// When present, the store is treated as atomic with the supplied ordering.
    pub ordering: Option<MemoryOrdering>,
    /// When true, the store is considered volatile.
    pub volatile: bool,
}

impl Instruction for MStore {
    fn flags(&self) -> InstructionFlags {
        InstructionFlags::MEMORY
    }

    fn operands(&self) -> impl Iterator<Item = &Operand> {
        [&self.addr, &self.value].into_iter()
    }

    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        [&mut self.addr, &mut self.value].into_iter()
    }

    fn referenced_types(&self) -> impl Iterator<Item = Typeref> {
        std::iter::empty()
    }

    fn referenced_types_mut(&mut self) -> impl Iterator<Item = &mut Typeref> {
        std::iter::empty()
    }

    fn destination_type(&self) -> Option<Typeref> {
        None
    }
}

/// Alloca instruction: allocate memory on the stack.
///
/// The allocated memory is automatically freed when the function returns. This is
/// useful for creating local variables with dynamic sizes and for mutability. In
/// theory one could use the LLVM `phi` instruction to create SSA values that
/// change over time, but in practice this is cumbersome and is easier to allocate
/// on the stack and let the optimizer handle promoting to registers if possible.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct MAlloca {
    /// Destination SSA name receiving the pointer to the allocated storage.
    pub dest: Name,
    /// Type of each element in the allocation.
    pub ty: Typeref,
    /// Number of elements to allocate (evaluated at run time).
    pub count: Operand,
    /// Optional byte alignment for the allocation.
    pub alignement: Option<u32>,
}

impl Instruction for MAlloca {
    fn flags(&self) -> InstructionFlags {
        InstructionFlags::empty()
    }

    fn operands(&self) -> impl Iterator<Item = &Operand> {
        std::iter::once(&self.count)
    }

    fn destination(&self) -> Option<Name> {
        Some(self.dest)
    }

    fn set_destination(&mut self, name: Name) {
        self.dest = name;
    }

    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        std::iter::once(&mut self.count)
    }

    fn destination_type(&self) -> Option<Typeref> {
        Some(self.ty)
    }

    fn referenced_types(&self) -> impl Iterator<Item = Typeref> {
        std::iter::once(self.ty)
    }

    fn referenced_types_mut(&mut self) -> impl Iterator<Item = &mut Typeref> {
        std::iter::once(&mut self.ty)
    }
}

/// `getelementptr` instruction is used to get the address of a sub-element
///
/// It performs address calculation only and does not access memory. The
/// instruction can also be used to calculate a vector of such addresses
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct MGetElementPtr {
    /// Destination SSA name receiving the computed address.
    pub dest: Name,
    /// Resulting pointer type (always `ptr`).
    pub ty: Typeref,
    /// Type to interpret `base` as pointing to.
    pub in_ty: Typeref,
    /// Base pointer operand.
    pub base: Operand,
    /// Index operands applied successively to `base`.
    pub indices: Vec<Operand>,
}

impl Instruction for MGetElementPtr {
    fn flags(&self) -> InstructionFlags {
        InstructionFlags::SIMPLE
    }

    fn operands(&self) -> impl Iterator<Item = &Operand> {
        std::iter::once(&self.base).chain(self.indices.iter())
    }

    fn destination(&self) -> Option<Name> {
        Some(self.dest)
    }

    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        std::iter::once(&mut self.base).chain(self.indices.iter_mut())
    }

    fn set_destination(&mut self, name: Name) {
        self.dest = name;
    }

    fn destination_type(&self) -> Option<Typeref> {
        Some(self.ty)
    }

    fn referenced_types(&self) -> impl Iterator<Item = Typeref> {
        std::iter::once(self.ty).chain(std::iter::once(self.in_ty))
    }

    fn referenced_types_mut(&mut self) -> impl Iterator<Item = &mut Typeref> {
        std::iter::once(&mut self.ty).chain(std::iter::once(&mut self.in_ty))
    }
}

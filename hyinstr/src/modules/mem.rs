//! Memory operations
//!
//! Load and store instructions with alignment, volatility, and optional
//! atomic ordering semantics compatible with common language memory models
//! (C++/Java). The exact effects of `MemoryOrdering` follow the referenced
//! specifications; only user‑visible controls are documented here.
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{
    modules::{
        Instruction,
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
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum MemoryOrdering {
    Unordered,
    Monotonic,
    Acq,
    Rel,
    AcqRel,
    SeqCst,
}

impl MemoryOrdering {
    pub fn to_string(&self) -> &'static str {
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
pub struct MLoad {
    pub dest: Name,
    pub ty: Typeref,
    pub addr: Operand,
    pub alignment: Option<u32>,

    /// A notable distinction with LLVM's memory model is that Hyperion does
    /// not allow syncscope('singlethread') operations; all atomic operations
    /// are assumed to be cross‑thread unless the access is non‑atomic.
    pub ordering: Option<MemoryOrdering>,
    pub volatile: bool,
}

impl Instruction for MLoad {
    fn operands(&self) -> impl Iterator<Item = &Operand> {
        std::iter::once(&self.addr)
    }

    fn destination(&self) -> Option<Name> {
        Some(self.dest)
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
/// Store a value to memory.
///
/// When `volatile` is true, the operation is prevented from being removed or
/// merged by typical optimizations. If an `ordering` other than `Unordered`
/// is specified, the store is considered atomic with the given ordering.
pub struct MStore {
    pub addr: Operand,
    pub value: Operand,
    pub alignment: Option<u32>,

    /// A notable distinction with LLVM's memory model is that Hyperion does
    /// not allow syncscope('singlethread') operations; all atomic operations
    /// are assumed to be cross‑thread unless the access is non‑atomic.
    pub ordering: Option<MemoryOrdering>,
    pub volatile: bool,
}

impl Instruction for MStore {
    fn operands(&self) -> impl Iterator<Item = &Operand> {
        [&self.addr, &self.value].into_iter()
    }
}

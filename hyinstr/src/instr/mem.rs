use crate::{instr::int::IOp, name::Name, types::aggregate::TypeRef};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Certain atomic instructions take ordering paramters that
/// determine which other atomic instructions on the same addr they
/// synchronize with. These semantics implement the Java or C++ memory
/// models; If there descriptions aren't precise enough, check those specs
/// (see specs references on [cppreference](https://en.cppreference.com/w/cpp/atomic/memory_order)).
/// You can also check LLVM's documentation on [Ordering](https://llvm.org/docs/LangRef.html#atomic-memory-ordering) for more details.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum MemoryOrdering {
    Unordered,
    Monotonic,
    Acq,
    Rel,
    AcqRel,
    SeqCst,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MLoad {
    pub addr: IOp,
    pub dst: Name,
    pub loaded_ty: TypeRef,
    pub volatile: bool,
    /// A notable distinction with LLVM's memory model is that hyperion
    /// does not allow syncscope('singlethread') operations; all atomic operations
    /// are assumed to be cross-thread unless specified as non-atomic (i.e., this
    /// field is None).
    pub atomicity: Option<MemoryOrdering>,
    pub alignment: u32,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MStore {
    pub addr: IOp,
    pub value: IOp, // TODO: Fixme
    pub volatile: bool,
    /// A notable distinction with LLVM's memory model is that hyperion
    /// does not allow syncscope('singlethread') operations; all atomic operations
    /// are assumed to be cross-thread unless specified as non-atomic (i.e., this
    /// field is None).
    pub atomicity: Option<MemoryOrdering>,
    pub alignment: u32,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MAlloca {
    pub allocated_type: TypeRef,
    pub num_elements: IOp,
    pub dst: Name,
    pub alignment: u32,
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct GetElementPtr {
    pub addr: IOp,
    pub indices: Vec<IOp>,
    pub dst: Name,
    pub in_bounds: bool,
    pub source_element_type: TypeRef,
}

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
use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use crate::{
    consts::AnyConst,
    modules::{
        instructions::{HyInstr, Instruction},
        operand::{Label, Name, Operand},
        symbol::{ExternalFunction, FunctionPointer, FunctionPointerType},
        terminator::Trap,
    },
    types::{TypeRegistry, Typeref, primary::WType},
    utils::Error,
};
use petgraph::prelude::DiGraphMap;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoEnumIterator};
use uuid::Uuid;

pub mod fmt;
pub mod instructions;
pub mod operand;
#[cfg(feature = "chumsky")]
pub mod parser;
pub mod symbol;
pub mod terminator;

/// All Global Variables and Functions have one of the following types of linkage:
#[derive(Debug, Default, Clone, Copy, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub enum Linkage {
    /// Global values with `Linkage::private` linkage are only directly accessible by objects in the current module.
    ///
    /// In particular, linking code into a module with a private global value may cause the private to be renamed
    /// as necessary to avoid collisions. Because the symbol is private to the module, all references can be updated.
    ///
    /// This doesn’t show up in any symbol table in the object file.
    #[default]
    Private,

    /// Similar to `Linkage::private`, but the value shows as a local symbol (STB_LOCAL in the case of ELF) in the object file.
    ///
    /// This corresponds to the notion of the ‘static’ keyword in C.
    Internal,

    /// Global values with `Linkage::external` linkage may be referenced by other modules,
    /// and may also be defined in other modules.
    External,
}

/// All Global Variables and Functions have one of the following visibility styles:
///
///
/// Note: A symbol with internal or private linkage must have default visibility.
#[derive(Debug, Default, Clone, Copy, Hash, PartialEq, Eq, EnumIter)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub enum Visibility {
    /// Default visibility
    ///
    /// On targets that use the ELF object file format, default visibility means that the declaration is visible to other modules
    /// and, in shared libraries, means that the declared entity may be overridden. On Darwin, default visibility means that the
    /// declaration is visible to other modules. On XCOFF, default visibility means no explicit visibility bit will be set and whether
    /// the symbol is visible (i.e “exported”) to other modules depends primarily on export lists provided to the linker. Default
    /// visibility corresponds to “external linkage” in the language.
    Default,

    /// Hidden visibility
    ///
    /// Two declarations of an object with hidden visibility refer to the same object if they are in the same shared object. Usually,
    /// hidden visibility indicates that the symbol will not be placed into the dynamic symbol table, so no other module (executable
    /// or shared library) can reference it directly.
    #[default]
    Hidden,

    /// Protected visibility
    ///
    /// On ELF, protected visibility indicates that the symbol will be placed in the dynamic symbol table, but that references within
    /// the defining module will bind to the local symbol. That is, the symbol cannot be overridden by another module.
    Protected,
}

impl Visibility {
    pub fn to_str(&self) -> &'static str {
        match self {
            Visibility::Default => "default",
            Visibility::Hidden => "hidden",
            Visibility::Protected => "protected",
        }
    }
}

impl std::str::FromStr for Visibility {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Visibility::iter().find(|v| v.to_str() == s).ok_or(())
    }
}

/// LLVM functions, calls and invokes can all have an optional calling convention specified for the call. The calling convention of any pair
/// of dynamic caller/callee must match, or the behavior of the program is undefined. The following calling conventions are supported by LLVM,
/// and more may be added in the future:
#[derive(Debug, Default, Clone, Copy, Hash, PartialEq, Eq, EnumIter)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub enum CallingConvention {
    /// The C calling convention
    ///
    /// This calling convention (the default if no other calling convention is specified) matches the target C calling conventions.
    /// This calling convention supports varargs function calls and tolerates some mismatch in the declared prototype and implemented
    /// declaration of the function (as does normal C).
    #[default]
    C,

    /// The fast calling convention
    ///
    /// This calling convention attempts to make calls as fast as possible (e.g., by passing things in registers). This calling convention
    /// allows the target to use whatever tricks it wants to produce fast code for the target, without having to conform to an externally
    /// specified ABI (Application Binary Interface). Tail calls can only be optimized when this, the tailcc, the GHC or the HiPE convention
    /// is used. This calling convention does not support varargs and requires the prototype of all callees to exactly match the prototype
    /// of the function definition.
    FastC,

    /// The cold calling convention
    ///
    /// This calling convention attempts to make code in the caller as efficient as possible under the assumption that the call is not
    /// commonly executed. As such, these calls often preserve all registers so that the call does not break any live ranges in the
    /// caller side. This calling convention does not support varargs and requires the prototype of all callees to exactly match the
    /// prototype of the function definition. Furthermore the inliner doesn’t consider such function calls for inlining.
    ColdC,

    /// GHC calling convention
    ///
    /// Implemented for use by the Glasgow Haskell Compiler. Passes as many
    /// arguments in registers as possible and disables many callee-saved
    /// registers; supports tail calls when both caller and callee use it.
    GhcC,

    /// HiPE calling convention
    ///
    /// Implemented for the High-Performance Erlang (HiPE) compiler. Uses
    /// more registers for argument passing and defines no callee-saved
    /// registers. Supports tail call optimization when caller and callee
    /// both use it.
    HipeC,

    /// Dynamic calling convention for code patching (anyregcc)
    ///
    /// Forces call arguments into registers but allows them to be dynamically
    /// allocated. Currently intended for use with patchpoints/stack maps.
    AnyRegC,

    /// PreserveMost calling convention
    ///
    /// Behaves like the C calling convention for argument/return passing but
    /// preserves a larger set of registers to minimize caller save/restore.
    PreserveMostC,

    /// PreserveAll calling convention
    ///
    /// Like PreserveMost but preserves an even larger set of registers
    /// (including many floating-point registers on supported targets).
    PreserveAllC,

    /// PreserveNone calling convention
    ///
    /// Does not preserve any general-purpose registers. All GP registers are
    /// caller-saved; non-GP registers (e.g., floating point) follow the
    /// platform's standard C convention.
    PreserveNoneC,

    /// CXX_FAST_TLS calling convention for C++ TLS access functions
    ///
    /// Minimizes overhead in the caller by preserving registers used on the
    /// fast path of TLS access functions. Platform-specific preserved set.
    CxxFastTlsC,

    /// Tail-call-optimized calling convention
    ///
    /// Equivalent to fastcc but guarantees tail call optimization when
    /// possible. Does not support varargs and requires exact prototype match.
    TailC,

    /// Swift calling convention
    ///
    /// Used by the Swift language. Target-specific details govern extra
    /// return registers and ABI choices (see platform docs).
    SwiftC,

    /// Swift tail-callable convention
    ///
    /// Like `SwiftC` but callee pops the argument area of the stack to
    /// permit mandatory tail calls.
    SwiftTailC,

    /// Control Flow Guard check calling convention
    ///
    /// Used for the Windows CFGuard check function inserted before indirect
    /// calls. The register used to pass the target is architecture-specific.
    CfguardCheckC,

    /// Numbered/target-specific calling convention (cc &lt;n&gt;)
    ///
    /// Allows target-specific calling conventions to be referenced by
    /// number. Targets reserve numbers starting at 64 for custom conventions.
    Numbered(u32),
}

impl CallingConvention {
    pub fn to_string(&self) -> Cow<'static, str> {
        match self {
            CallingConvention::C => "cc".into(),
            CallingConvention::FastC => "fastcc".into(),
            CallingConvention::ColdC => "coldcc".into(),
            CallingConvention::GhcC => "ghccc".into(),
            CallingConvention::HipeC => "hipecc".into(),
            CallingConvention::AnyRegC => "anyregcc".into(),
            CallingConvention::PreserveMostC => "preservemostcc".into(),
            CallingConvention::PreserveAllC => "preserveallcc".into(),
            CallingConvention::PreserveNoneC => "preservenonecc".into(),
            CallingConvention::CxxFastTlsC => "cxx_fast_tlscc".into(),
            CallingConvention::TailC => "tailcc".into(),
            CallingConvention::SwiftC => "swiftcc".into(),
            CallingConvention::SwiftTailC => "swifttailcc".into(),
            CallingConvention::CfguardCheckC => "cfguard_checkcc".into(),
            CallingConvention::Numbered(n) => format!("cc{}", n).into(),
        }
    }
}

impl std::str::FromStr for CallingConvention {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match CallingConvention::iter()
            .filter(|x| !matches!(x, CallingConvention::Numbered(_)))
            .find(|cc| cc.to_string().as_ref() == s)
        {
            Some(cc) => Ok(cc),
            None => {
                if let Some(num_str) = s.strip_prefix("cc")
                    && let Ok(num) = num_str.parse::<u32>()
                {
                    return Ok(CallingConvention::Numbered(num));
                }

                Err(())
            }
        }
    }
}

/// Reference to a specific instruction within a function.
///
/// This structure identifies an instruction by the basic block label it resides in
/// and the index of the instruction within that block.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct InstructionRef {
    /// Label of the basic block containing the instruction.
    pub block: Label,
    /// Zero-based position of the instruction within the block.
    pub index: u32,
    /// Reserved for other use (check [`super::proof::AttachedFunction`] for examples).
    pub reserved: u64,
}

impl From<(Label, usize)> for InstructionRef {
    fn from((block, index): (Label, usize)) -> Self {
        Self {
            block,
            index: index as u32,
            reserved: 0,
        }
    }
}

impl From<InstructionRef> for (Label, usize) {
    fn from(reference: InstructionRef) -> Self {
        (reference.block, reference.index as usize)
    }
}

/// A basic block within a function, containing a sequence of instructions
/// and ending with a control flow terminator.
///
/// Each basic block is uniquely identified by a UUID.
///
/// This structure allows to define a group of instructions that execute
/// sequentially, followed by a control flow instruction that determines
/// the next block to execute. This structure allows for the representation
/// of complex control flow within functions.
#[derive(Debug, Clone, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct BasicBlock {
    /// Unique block label.
    pub label: Label,
    /// Linear list of instructions executed before the terminator.
    pub instructions: Vec<HyInstr>,
    /// Control-flow terminator ending the block.
    pub terminator: terminator::HyTerminator,
}

impl BasicBlock {
    /// Get the label of the basic block.
    pub fn label(&self) -> Label {
        self.label
    }

    /// Create a [`FunctionInstructionReference`] for the instruction at the given index.
    pub fn instruction_reference(&self, index: usize) -> InstructionRef {
        assert!(
            index < self.instructions.len() && index <= u32::MAX as usize,
            "Instruction index out of bounds for basic block (label: {:?}, index: {})",
            self.label,
            index
        );

        InstructionRef {
            block: self.label,
            index: index as u32,
            reserved: 0,
        }
    }
}

/// A function made of basic blocks and parameter metadata.
///
/// A `Function` owns its control‑flow graph (`body`) and carries optional
/// metadata such as a display `name`, `visibility`, and a `CallingConvention`.
/// Parameters are represented as a list of `(Name, Typeref)` pairs.
///
/// By convention the entrypoint is the basic block with the [`Uuid::nil()`] UUID.
///
/// We distinguish between "meta-functions" and regular functions. Meta-functions are used
/// for verification or analysis purposes and may contain meta-instructions and operands.
/// They cannot be executed directly but serve as specifications or annotations for other functions.
///
/// Meta-functions should never loop and always terminate, they may contain loop-like structures
/// for the purpose of expressing invariants over iterations, but these loops must be bounded
/// and analyzable. Regular functions, on the other hand, represent executable code and may
/// contain arbitrary control flow, including loops and recursion.
///
#[derive(Debug, Clone, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct Function {
    /// The unique identifier (UUID) of the function.
    pub uuid: Uuid,
    /// The display name of the function, if any (debugging purposes).
    pub name: Option<String>,
    /// The list of parameters for the function (as `(Name, Typeref)` pairs).
    pub params: Vec<(Name, Typeref)>,
    /// The return type of the function. `None` indicates `void` return type.
    pub return_type: Option<Typeref>,
    /// The body of the function, represented as a mapping from basic block labels to basic blocks.
    pub body: BTreeMap<Label, BasicBlock>,
    /// The visibility of the function (ignored for meta-functions).
    pub visibility: Option<Visibility>,
    /// The linkage of the function (ignored for meta-functions).
    pub cconv: Option<CallingConvention>,
    /// The set of wildcard types used in the function.
    pub wildcard_types: BTreeSet<WType>,
    /// Indicates whether this function is a meta-function (i.e., used for verification or analysis purposes).
    pub meta_function: bool,
    /// If this function was derived from another, holds the source function UUID.
    pub derived_from: Option<Uuid>,
}

impl Default for Function {
    fn default() -> Self {
        Self {
            uuid: Uuid::new_v4(),
            name: None,
            params: vec![],
            return_type: None,
            body: [(
                Label::NIL,
                BasicBlock {
                    label: Label::NIL,
                    instructions: vec![],
                    terminator: terminator::HyTerminator::Trap(Trap),
                },
            )]
            .into_iter()
            .collect(),
            visibility: None,
            cconv: None,
            wildcard_types: Default::default(),
            meta_function: false,
            derived_from: Default::default(),
        }
    }
}

impl Function {
    /// Maximum allowed size for a basic block (number of instructions).
    pub const MAX_INSTR_PER_BLOCK: usize = 65536;

    /// Maximum allowed number of basic blocks in a function.
    pub const MAX_BLOCK_PER_FUNC: usize = 65536;

    /// Maximum allowed number of instructions in a function.
    pub const MAX_INSTR_PER_FUNC: usize = 1_000_000;

    /// Maximum allowed number of parameters in a function.
    pub const MAX_PARAMS_PER_FUNC: usize = 4096;

    /// Maximum allowed number of wildcard types in a function.
    pub const MAX_WILDCARD_TYPES_PER_FUNC: usize = 256;

    fn generate_wildcard_types(&self, wildcards: &mut BTreeSet<WType>) {
        // Scan parameters and instructions for wildcard types
        wildcards.clear();

        // Verify parameters
        for (_, typeref) in &self.params {
            if let Some(wt) = typeref.try_as_wildcard() {
                wildcards.insert(wt);
            }
        }

        // Iterate over all instructions to find all types referenced
        for bb in self.body.values() {
            for instr in &bb.instructions {
                for typeref in instr.referenced_types() {
                    if let Some(wt) = typeref.try_as_wildcard() {
                        wildcards.insert(wt);
                    }
                }
            }

            for typeref in bb.terminator.referenced_types() {
                if let Some(wt) = typeref.try_as_wildcard() {
                    wildcards.insert(wt);
                }
            }
        }
    }

    fn verify_wildcards_soundness(&self) -> Result<(), Error> {
        // Verify that all wildcard types used in parameters and instructions
        // are declared in `wildcard_types`.
        let mut generated = BTreeSet::new();
        self.generate_wildcard_types(&mut generated);

        if generated != self.wildcard_types {
            return Err(Error::UnsoundWildcardTypes {
                function: self.name.clone().unwrap_or_else(|| self.uuid.to_string()),
                expected: self
                    .wildcard_types
                    .iter()
                    .map(|wt| wt.to_string())
                    .collect(),
                found: generated.iter().map(|wt| wt.to_string()).collect(),
            });
        }

        Ok(())
    }

    fn verify_phi_first_instr_of_block(&self) -> Result<(), Error> {
        for bb in self.body.values() {
            let mut found_non_phi = false;
            for instr in &bb.instructions {
                if instr.is_phi() {
                    if found_non_phi {
                        return Err(Error::PhiNotFirstInstruction { block: bb.label });
                    }
                } else {
                    found_non_phi = true;
                }
            }
        }
        Ok(())
    }

    fn verify_target_soundness(&self) -> Result<(), Error> {
        for bb in self.body.values() {
            // Check terminator does not refer to non-existing basic blocks
            for (target_label, _) in bb.terminator.iter_targets() {
                if !self.body.contains_key(&target_label) {
                    return Err(Error::UndefinedBasicBlock {
                        function: self.name.clone().unwrap_or_else(|| self.uuid.to_string()),
                        label: target_label,
                    });
                }
            }
        }
        Ok(())
    }

    fn verify_no_meta_instruction(&self) -> Result<(), Error> {
        for bb in self.body.values() {
            for instr in &bb.instructions {
                if instr.is_meta_instruction() {
                    return Err(Error::MetaInstructionNotAllowed {
                        function: self.name.clone().unwrap_or_else(|| self.uuid.to_string()),
                        instruction: format!("{:?}", instr),
                    });
                }
            }
        }
        Ok(())
    }

    fn verify_size_constraints(&self) -> Result<(), Error> {
        if self.body.len() > Self::MAX_BLOCK_PER_FUNC {
            return Err(Error::FunctionTooManyBlocks {
                function: self.name.clone().unwrap_or_else(|| self.uuid.to_string()),
                count: self.body.len(),
                max: Self::MAX_BLOCK_PER_FUNC,
            });
        }

        let mut instr_count = 0usize;
        for (label, bb) in &self.body {
            if bb.instructions.len() > Self::MAX_INSTR_PER_BLOCK {
                return Err(Error::BasicBlockTooLarge {
                    function: self.name.clone().unwrap_or_else(|| self.uuid.to_string()),
                    block: *label,
                    count: bb.instructions.len(),
                    max: Self::MAX_INSTR_PER_BLOCK,
                });
            }

            instr_count += bb.instructions.len();
        }

        if instr_count > Self::MAX_INSTR_PER_FUNC {
            return Err(Error::FunctionTooManyInstructions {
                function: self.name.clone().unwrap_or_else(|| self.uuid.to_string()),
                count: instr_count,
                max: Self::MAX_INSTR_PER_FUNC,
            });
        }

        if self.params.len() > Self::MAX_PARAMS_PER_FUNC {
            return Err(Error::FunctionTooManyArguments {
                function: self.name.clone().unwrap_or_else(|| self.uuid.to_string()),
                count: self.params.len(),
                max: Self::MAX_PARAMS_PER_FUNC,
            });
        }

        if self.wildcard_types.len() > Self::MAX_WILDCARD_TYPES_PER_FUNC {
            return Err(Error::FunctionTooManyWildcardTypes {
                function: self.name.clone().unwrap_or_else(|| self.uuid.to_string()),
                count: self.wildcard_types.len(),
                max: Self::MAX_WILDCARD_TYPES_PER_FUNC,
            });
        }

        Ok(())
    }

    fn verify_ssa_soundness(&self) -> Result<(), Error> {
        let mut defined_names = BTreeSet::new();

        // 1. Construct defined_names
        for (name, _) in self.params.iter() {
            if !defined_names.insert(*name) {
                return Err(Error::DuplicateSSAName { duplicate: *name });
            }
        }

        for bb in self.body.values() {
            for instr in &bb.instructions {
                if let Some(dest) = instr.destination()
                    && !defined_names.insert(dest)
                {
                    return Err(Error::DuplicateSSAName { duplicate: dest });
                }
            }
        }

        // 2. Ensure all operands refer to defined names
        for bb in self.body.values() {
            for instr in &bb.instructions {
                for name in instr.dependencies() {
                    if !defined_names.contains(&name) {
                        return Err(Error::UndefinedSSAName { undefined: name });
                    }
                }
            }

            for name in bb.terminator.dependencies() {
                if !defined_names.contains(&name) {
                    return Err(Error::UndefinedSSAName { undefined: name });
                }
            }
        }

        Ok(())
    }

    /// Check whether the function should be treated as a meta-function.
    ///
    /// A function is considered a meta-function if it contains any meta-instructions.
    /// Meta-functions are used for verification or analysis purposes and cannot be
    /// executed directly, hence the distinction.
    ///
    pub fn should_be_meta_function(&self) -> bool {
        // If any instruction is a meta-instruction, the function is meta
        for bb in self.body.values() {
            for instr in &bb.instructions {
                if instr.is_meta_instruction() {
                    return true;
                }
            }
        }

        false
    }

    /// Generate wildcard types from parameters and instructions.
    pub fn generate_wildcards(&mut self) {
        let mut placeholder = BTreeSet::new(); // Doesn't allocate anything on its own
        std::mem::swap(&mut self.wildcard_types, &mut placeholder);
        self.generate_wildcard_types(&mut placeholder);
        std::mem::swap(&mut self.wildcard_types, &mut placeholder);
    }

    /// Returns whether the function is incomplete (i.e., has unresolved wildcard types).
    pub fn is_incomplete(&self) -> bool {
        !self.wildcard_types.is_empty()
    }

    /// Find next available [`Name`] for a parameter.
    pub fn next_available_name(&self) -> Name {
        let mut max_index = 0;
        for (name, _) in &self.params {
            max_index = max_index.max(name.0);
        }

        for bb in self.body.values() {
            for instr in &bb.instructions {
                if let Some(dest) = instr.destination() {
                    max_index = max_index.max(dest.0);
                }
            }
        }

        Name(max_index + 1)
    }

    /// Find next available [`Label`] for a basic block.
    ///
    /// Notice that for entry block you should use [`Label::NIL`].
    pub fn next_available_label(&self) -> Label {
        let mut max_index = 1;
        for label in self.body.keys() {
            max_index = max_index.max(label.0);
        }
        Label(max_index + 1)
    }

    /// Verify the soundness of the function.
    ///
    /// This method performs a series of checks to ensure the integrity and correctness
    /// of the function's structure and contents. It verifies:
    /// - Wildcard types are sound.
    /// - No meta-operands are present in non-meta functions.
    /// - No meta-instructions are present in non-meta functions.
    /// - Phi instructions are the first instructions in their respective blocks.
    /// - Target basic blocks referenced by terminators exist.
    /// - SSA form is maintained (all names are defined before use).
    /// - Size constraints for blocks and functions are respected.
    /// - The existence of an entry block.
    ///
    pub fn verify(&self) -> Result<(), Error> {
        self.verify_wildcards_soundness()?;
        if !self.meta_function {
            self.verify_no_meta_instruction()?;
        }
        self.verify_phi_first_instr_of_block()?;
        self.verify_target_soundness()?;
        self.verify_ssa_soundness()?;
        self.verify_size_constraints()?;

        // Ensure existence of entry block
        if !self.body.contains_key(&Label::NIL) {
            return Err(Error::MissingEntryBlock);
        }

        // TODO: Verify that all SSA names are defined before use (topological order)
        Ok(())
    }

    /// Perform type checking on the function.
    ///
    /// See [`super::types::checker::type_check`] function for more details.
    pub fn type_check(&self, type_registry: &TypeRegistry) -> Result<(), Error> {
        super::types::checker::type_check(
            type_registry,
            self.params.iter().copied(),
            self.body.iter().flat_map(|x| x.1.instructions.iter()),
            self.body.iter().map(|x| &x.1.terminator),
            self.return_type,
        )
    }

    /// Normalize SSA names in the function to ensure uniqueness and sequential ordering.
    ///
    /// This method remaps all SSA names used in the function's parameters and instructions
    /// to a new set of unique names starting from `Name(0)`. It ensures that each SSA name
    /// is used exactly once as a destination and that all operands refer to the newly assigned names.
    ///
    pub fn normalize_ssa(&mut self) {
        let mut name_mapping = BTreeMap::new();
        let mut next_name = Name(0);

        // Remap all SSA names in parameters
        for (name, _) in self.params.iter_mut() {
            let _output = name_mapping.insert(*name, next_name);
            debug_assert!(_output.is_none());
            *name = next_name;
            next_name += 1;
        }

        // For each instruction destination, allocate a new name if needed
        for bb in self.body.values_mut() {
            for instr in bb.instructions.iter_mut() {
                if let Some(dest) = instr.destination() {
                    let _output = name_mapping.insert(dest, next_name);
                    debug_assert!(_output.is_none());
                    instr.set_destination(next_name);
                    next_name += 1;
                }
            }
        }

        // Now remap all operands according to the mapping
        for bb in self.body.values_mut() {
            for instr in &mut bb.instructions {
                for op in instr.dependencies_mut() {
                    *op = name_mapping[op];
                }
            }

            for op in bb.terminator.dependencies_mut() {
                *op = name_mapping[op];
            }
        }
    }

    /// Retrieve instruction from a [`InstructionRef`].
    ///
    /// Returns `None` if the block or instruction index is invalid.
    ///
    pub fn get(&self, reference: InstructionRef) -> Option<&HyInstr> {
        self.body
            .get(&reference.block)
            .and_then(|bb| bb.instructions.get(reference.index as usize))
    }

    /// Analyzes the control flow of a function and constructs its control flow graph (CFG).
    ///
    /// The CFG is represented as a directed graph where nodes are basic block labels
    /// and edges represent possible control flow transitions between blocks. Each edge
    /// is annotated with an optional condition operand that determines whether the transition
    /// occurs.
    ///
    pub fn derive_function_flow(&self) -> DiGraphMap<Label, Option<Operand>> {
        let mut graph = DiGraphMap::with_capacity(self.body.len(), self.body.len() * 3);

        // Pass 1: Add all nodes
        for block_label in self.body.keys().copied() {
            graph.add_node(block_label);
        }

        // Pass 2: Add edges based on terminators
        for (block_label, block) in &self.body {
            block
                .terminator
                .iter_targets()
                .for_each(|(target_label, condition)| {
                    graph.add_edge(*block_label, target_label, condition.cloned());
                });
        }

        graph
    }

    /// Derive the dest-map, for each SSA name, find the instruction that defines it.
    ///
    /// You can use this to quickly lookup the instruction that defines a particular SSA name.
    /// Notice that some [`Name`]s may not be present in the map, typically function parameters.
    ///
    pub fn derive_dest_map(&self) -> BTreeMap<Name, InstructionRef> {
        let mut dest_map = BTreeMap::new();

        for (block_label, block) in &self.body {
            for (instr_index, instr) in block.instructions.iter().enumerate() {
                if let Some(dest) = instr.destination() {
                    dest_map.insert(dest, InstructionRef::from((*block_label, instr_index)));
                }
            }
        }

        dest_map
    }

    /// Derive a list of all [`InstructionRef`]s based on a predicate
    pub fn gather_instructions_by_predicate<F>(&self, predicate: F) -> Vec<InstructionRef>
    where
        F: Fn(&HyInstr) -> bool,
    {
        let mut references = Vec::new();

        for (block_label, block) in &self.body {
            for (instr_index, instr) in block.instructions.iter().enumerate() {
                if predicate(instr) {
                    references.push(InstructionRef::from((*block_label, instr_index)));
                }
            }
        }

        references
    }

    /// Iterate over all instructions in the function.
    pub fn iter(&self) -> impl Iterator<Item = (&HyInstr, InstructionRef)> {
        self.body.iter().flat_map(|(block_label, block)| {
            block
                .instructions
                .iter()
                .enumerate()
                .map(move |(instr_index, instr)| {
                    (instr, InstructionRef::from((*block_label, instr_index)))
                })
        })
    }

    /// Iterate mutably over all instructions in the function.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&mut HyInstr, InstructionRef)> {
        self.body.iter_mut().flat_map(|(block_label, block)| {
            block
                .instructions
                .iter_mut()
                .enumerate()
                .map(move |(instr_index, instr)| {
                    (instr, InstructionRef::from((*block_label, instr_index)))
                })
        })
    }

    /// Retrieve an instruction from its SSA destination name.
    pub fn get_instruction_by_dest(&self, name: Name) -> Option<&HyInstr> {
        for block in self.body.values() {
            for instr in &block.instructions {
                if let Some(dest) = instr.destination()
                    && dest == name
                {
                    return Some(instr);
                }
            }
        }
        None
    }

    /// Get analysis context for the function.
    pub fn analyze(self: Arc<Self>) -> FunctionAnalysis {
        FunctionAnalysis {
            cfg: self.derive_function_flow(),
            dest_map: self.derive_dest_map(),
            function: self,
        }
    }

    /// Remap types in the function according to the provided mapping.
    pub fn remap_types(&mut self, mapping: &BTreeMap<Typeref, Typeref>) {
        // Remap parameter types
        for (_, typeref) in self.params.iter_mut() {
            if let Some(new_type) = mapping.get(typeref) {
                *typeref = *new_type;
            }
        }

        // Remap return type
        if let Some(ret_type) = &self.return_type
            && let Some(new_type) = mapping.get(ret_type)
        {
            self.return_type = Some(*new_type);
        }

        // Remap types in each instruction
        for bb in self.body.values_mut() {
            for instr in bb.instructions.iter_mut() {
                instr.remap_types(|ty| mapping.get(&ty).cloned());
            }

            bb.terminator.remap_types(|ty| mapping.get(&ty).cloned());
        }
    }
}

/// Analyze context for a function.
///
/// This contains a list of acceleration structures to speed up analysis and
/// ease lookups when dealing with functions.
#[derive(Debug, Clone)]
pub struct FunctionAnalysis {
    /// The function being analyzed.
    pub function: Arc<Function>,
    /// The control flow graph of the function.
    pub cfg: DiGraphMap<Label, Option<Operand>>,
    /// The destination map of the function.
    pub dest_map: BTreeMap<Name, InstructionRef>,
}

/// A module containing defined functions and references to external ones.
///
/// `Module` acts as the compilation unit boundary for symbol visibility.
/// Functions defined here appear in `functions`; references to symbols not
/// defined locally are listed in `external_functions`.
#[derive(Debug, Default, Clone, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct Module {
    /// Defined functions keyed by their UUID.
    pub functions: BTreeMap<Uuid, Arc<Function>>,
    /// Declared external functions keyed by their UUID.
    pub external_functions: BTreeMap<Uuid, ExternalFunction>,
}

impl Module {
    /// Check that a particular function validly references only defined functions.
    ///
    /// Notice that recursive calls are allowed, that is to say that function that self-references
    /// are considered valid, even if the function is not defined in the module.
    pub fn verify_func(&self, function: &Function) -> Result<(), Error> {
        for bb in function.body.values() {
            for instr in &bb.instructions {
                // If operand is a external function ptr
                for op in instr.operands() {
                    if let Operand::Imm(AnyConst::FuncPtr(func_ptr)) = op {
                        match func_ptr {
                            FunctionPointer::Internal(uuid) => {
                                if uuid == &function.uuid {
                                    continue; // Recursive call are allowed, even if function not currently in the module
                                }

                                if !self.functions.contains_key(uuid) {
                                    return Err(Error::UndefinedInternalFunction {
                                        function: function
                                            .name
                                            .clone()
                                            .unwrap_or_else(|| function.uuid.to_string()),
                                        undefined: *uuid,
                                    });
                                }
                            }
                            FunctionPointer::External(uuid) => {
                                if !self.external_functions.contains_key(uuid) {
                                    return Err(Error::UndefinedExternalFunction {
                                        function: function
                                            .name
                                            .clone()
                                            .unwrap_or_else(|| function.uuid.to_string()),
                                        undefined: *uuid,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Find the UUID of a function by its name and type (internal or external).
    ///
    /// This operation is in O(n) in the number of functions in the module.
    ///
    /// Returns `None` if no function with the given name and type exists.
    pub fn find_function_uuid_by_name(
        &self,
        name: &str,
        func_type: FunctionPointerType,
    ) -> Option<FunctionPointer> {
        match func_type {
            FunctionPointerType::Internal => self
                .functions
                .values()
                .find(|f| f.as_ref().name.as_deref() == Some(name))
                .map(|f| FunctionPointer::Internal(f.as_ref().uuid)),
            FunctionPointerType::External => self
                .external_functions
                .values()
                .find(|f| f.name == name)
                .map(|f| FunctionPointer::External(f.uuid)),
        }
    }

    /// Find the UUID of an internal function by its name.
    ///
    /// This operation is in O(n) in the number of functions in the module.
    ///
    /// Returns `None` if no internal function with the given name exists.
    pub fn find_internal_function_uuid_by_name(&self, name: &str) -> Option<Uuid> {
        self.functions
            .values()
            .find(|f| f.as_ref().name.as_deref() == Some(name))
            .map(|f| f.as_ref().uuid)
    }

    /// Retrieve a particular function from its Uuid
    pub fn get_internal_function_by_uuid(&self, uuid: Uuid) -> Option<&Function> {
        self.functions.get(&uuid).map(|f| f.as_ref())
    }

    /// Retrieve a particular function from its Uuid (mutable)
    pub fn get_internal_function_by_uuid_mut(&mut self, uuid: Uuid) -> Option<&mut Function> {
        self.functions
            .get_mut(&uuid)
            .and_then(|arc| Arc::get_mut(arc))
    }

    /// Check each function in the module for SSA validity.
    pub fn verify(&self) -> Result<(), Error> {
        for func in self.functions.values() {
            let function = func.as_ref();
            function.verify()?;
            self.verify_func(function)?;
        }

        Ok(())
    }

    /// Type check each function in the module.
    pub fn type_check(&self, type_registry: &TypeRegistry) -> Result<(), Error> {
        for func in self.functions.values() {
            func.type_check(type_registry)?;
        }

        Ok(())
    }

    /// Remap types in the module according to the provided mapping.
    pub fn remap_types(&mut self, mapping: &BTreeMap<Typeref, Typeref>) {
        // Remap types in each function
        for func in self.functions.values_mut() {
            // Get mutable reference to the function
            let function = Arc::get_mut(func).expect(
                "Cannot remap types in function behind Arc; no other references should exist",
            );
            function.remap_types(mapping);
        }

        // Remap types in each external function
        for ext_func in self.external_functions.values_mut() {
            ext_func.remap_types(|ty| mapping.get(ty).cloned());
        }
    }
}

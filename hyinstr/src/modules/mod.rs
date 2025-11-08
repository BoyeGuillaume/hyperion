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
use std::collections::{BTreeMap, BTreeSet};

use crate::{
    consts::AnyConst,
    modules::{
        instructions::HyInstr,
        operand::{Label, Name, Operand},
        symbol::ExternalFunction,
    },
    types::{Typeref, primary::WType},
    utils::Error,
};
use petgraph::prelude::DiGraphMap;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod fp;
pub mod instructions;
pub mod int;
pub mod mem;
pub mod misc;
pub mod operand;
pub mod symbol;
pub mod terminator;

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

    /// Type of the destination SSA name if the instruction produces a result.
    fn destination_type(&self) -> Option<Typeref> {
        None
    }

    /// Any types referenced by this instruction.
    fn referenced_types(&self) -> impl Iterator<Item = Typeref>;

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
}

/// All Global Variables and Functions have one of the following types of linkage:
#[derive(Debug, Default, Clone, Copy, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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
#[derive(Debug, Default, Clone, Copy, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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

/// LLVM functions, calls and invokes can all have an optional calling convention specified for the call. The calling convention of any pair
/// of dynamic caller/callee must match, or the behavior of the program is undefined. The following calling conventions are supported by LLVM,
/// and more may be added in the future:
#[derive(Debug, Default, Clone, Copy, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
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

/// Reference to a specific instruction within a function.
///
/// This structure identifies an instruction by the basic block label it resides in
/// and the index of the instruction within that block.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct FunctionInstructionReference {
    pub block: Label,
    pub index: usize,
}

impl From<(Label, usize)> for FunctionInstructionReference {
    fn from((block, index): (Label, usize)) -> Self {
        Self { block, index }
    }
}

impl From<FunctionInstructionReference> for (Label, usize) {
    fn from(reference: FunctionInstructionReference) -> Self {
        (reference.block, reference.index)
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
pub struct BasicBlock {
    pub label: Label,
    pub instructions: Vec<HyInstr>,
    pub terminator: terminator::Terminator,
}

impl BasicBlock {
    /// Get the label of the basic block.
    pub fn label(&self) -> Label {
        self.label
    }

    /// Create a [`FunctionInstructionReference`] for the instruction at the given index.
    pub fn instruction_reference(&self, index: usize) -> FunctionInstructionReference {
        FunctionInstructionReference {
            block: self.label,
            index,
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
#[derive(Debug, Clone, Hash)]
pub struct Function {
    pub uuid: Uuid,
    pub name: Option<String>,
    pub params: Vec<(Name, Typeref)>,
    pub return_type: Option<Typeref>,
    pub body: BTreeMap<Label, BasicBlock>,
    pub visibility: Option<Visibility>,
    pub cconv: Option<CallingConvention>,
    pub wildcard_types: BTreeSet<WType>,
}

impl Function {
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

            // Terminator do not reference any types (technically condition are always i1)
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

    fn verify_no_meta_operands(&self) -> Result<(), Error> {
        for bb in self.body.values() {
            for instr in &bb.instructions {
                for operand in instr.operands() {
                    if let Operand::Meta(_) = operand {
                        return Err(Error::MetaOperandNotAllowed);
                    }
                }
            }
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
                if let Some(dest) = instr.destination() {
                    if !defined_names.insert(dest) {
                        return Err(Error::DuplicateSSAName { duplicate: dest });
                    }
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
            max_index = max_index.max(*name);
        }

        for bb in self.body.values() {
            for instr in &bb.instructions {
                if let Some(dest) = instr.destination() {
                    max_index = max_index.max(dest);
                }
            }
        }

        max_index + 1
    }

    /// Find next available [`Label`] for a basic block.
    pub fn next_available_label(&self) -> Label {
        let mut max_index = 0;
        for label in self.body.keys() {
            max_index = max_index.max(label.0);
        }
        Label(max_index + 1)
    }

    /// Verify SSA form:
    /// 1) Each operand refers to a defined name.
    /// 2) Each name is defined exactly once.
    pub fn check_ssa(&self) -> Result<(), Error> {
        self.verify_wildcards_soundness()?;
        self.verify_no_meta_operands()?;
        self.verify_phi_first_instr_of_block()?;
        self.verify_target_soundness()?;
        self.verify_ssa_soundness()?;

        // Ensure existence of entry block
        if !self.body.contains_key(&Label::NIL) {
            return Err(Error::MissingEntryBlock);
        }

        // TODO: Verify that all SSA names are defined before use (topological order)
        Ok(())
    }

    /// Normalize the function by ensuring that all SSA names are sequentially
    /// numbered from zero upwards without gaps. Because of the use of `BTreeMap`
    /// for basic blocks, ordering is always deterministic.
    pub fn normalize_ssa(&mut self) {
        let mut name_mapping = BTreeMap::new();
        let mut next_name = 0;

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

    /// Retrieve instruction from a [`FunctionInstructionReference`].
    pub fn get(&self, reference: FunctionInstructionReference) -> Option<&HyInstr> {
        self.body
            .get(&reference.block)
            .and_then(|bb| bb.instructions.get(reference.index))
    }

    /// Analyzes the control flow of a function and constructs its control flow graph (CFG).
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

    /// Derive the dest-map, for each SSA name associate 1) the defining block label and 2) the instruction index within the block.
    pub fn derive_dest_map(&self) -> BTreeMap<Name, (Label, usize)> {
        let mut dest_map = BTreeMap::new();

        for (block_label, block) in &self.body {
            for (instr_index, instr) in block.instructions.iter().enumerate() {
                if let Some(dest) = instr.destination() {
                    dest_map.insert(dest, (*block_label, instr_index));
                }
            }
        }

        dest_map
    }

    /// Retrieve an instruction from its SSA destination name.
    pub fn get_instruction_by_dest(&self, name: Name) -> Option<&HyInstr> {
        for block in self.body.values() {
            for instr in &block.instructions {
                if let Some(dest) = instr.destination() {
                    if dest == name {
                        return Some(instr);
                    }
                }
            }
        }
        None
    }
}

/// A module containing defined functions and references to external ones.
///
/// `Module` acts as the compilation unit boundary for symbol visibility.
/// Functions defined here appear in `functions`; references to symbols not
/// defined locally are listed in `external_functions`.
#[derive(Debug, Clone, Hash)]
pub struct Module {
    pub functions: BTreeMap<Uuid, Function>,
    pub external_functions: BTreeMap<Uuid, ExternalFunction>,
}

impl Module {
    /// Check each function in the module for SSA validity.
    pub fn check_ssa(&self) -> Result<(), Error> {
        for function in self.functions.values() {
            function.check_ssa()?;
        }

        // Ensure all external/internal function references are defined
        for function in self.functions.values() {
            for bb in function.body.values() {
                for instr in &bb.instructions {
                    // If operand is a external function ptr
                    for op in instr.operands() {
                        if let Operand::Imm(AnyConst::FuncPtr(func_ptr)) = op {
                            match func_ptr {
                                symbol::FunctionPointer::Internal(uuid) => {
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
                                symbol::FunctionPointer::External(uuid) => {
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
        }

        Ok(())
    }
}

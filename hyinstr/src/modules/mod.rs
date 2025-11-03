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
    modules::{
        instructions::HyInstr,
        operand::{Label, Name, Operand},
        symbol::ExternalFunction,
    },
    types::Typeref,
    utils::Error,
};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod control_flow;
pub mod fp;
pub mod instructions;
pub mod int;
pub mod mem;
pub mod operand;
pub mod symbol;

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

    /// Update the destination SSA name for this instruction. No-op if the
    /// instruction does not produce a result.
    fn set_destination(&mut self, _name: Name) {}

    /// Mutably iterate over all input operands for this instruction.
    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand>;

    /// Convenience iterator over referenced SSA names (i.e., register
    /// operands). Immediates and labels are ignored.
    fn name_dependencies(&self) -> impl Iterator<Item = Name> {
        self.operands().filter_map(|op| match op {
            Operand::Reg(reg) => Some(*reg),
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
    pub uuid: Uuid,
    pub instructions: Vec<HyInstr>,
    pub terminator: control_flow::Terminator,
}

impl BasicBlock {
    /// Get the label of the basic block.
    pub fn label(&self) -> Label {
        Label(self.uuid)
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
    pub body: BTreeMap<Uuid, BasicBlock>,
    pub visibility: Option<Visibility>,
    pub cconv: Option<CallingConvention>,
}

impl Function {
    /// Find next available [`Name`] for a parameter.
    pub fn next_available_name(&self) -> Name {
        let mut max_index = 0;
        for (name, _) in &self.params {
            max_index = max_index.max(*name);
        }

        for bb in self.body.values() {
            for instr in &bb.instructions {
                for op in instr.operands() {
                    if let Operand::Reg(name) = op {
                        max_index = max_index.max(*name);
                    }
                }
            }
        }

        max_index + 1
    }

    /// Verify SSA form:
    /// 1) Each operand refers to a defined name.
    /// 2) Each name is defined exactly once.
    pub fn check_ssa(&self) -> Result<(), Error> {
        let mut defined_names = BTreeSet::new();

        // Ensure existence of entry block
        if !self.body.contains_key(&Uuid::nil()) {
            return Err(Error::MissingEntryBlock);
        }

        // Construct a set of defined names from parameters
        for (name, _) in self.params.iter() {
            if !defined_names.insert(*name) {
                return Err(Error::DuplicateSSAName { duplicate: *name });
            }
        }

        // Same for each instruction destination of each basic block
        for bb in self.body.values() {
            for instr in &bb.instructions {
                if let Some(dest) = instr.destination() {
                    if !defined_names.insert(dest) {
                        return Err(Error::DuplicateSSAName { duplicate: dest });
                    }
                }
            }
        }

        // Now ensure all operands refer to defined names
        for bb in self.body.values() {
            for instr in &bb.instructions {
                for name in instr.name_dependencies() {
                    if !defined_names.contains(&name) {
                        return Err(Error::UndefinedSSAName { undefined: name });
                    }
                }
            }
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
        for bb in self.body.values() {
            for instr in &bb.instructions {
                if let Some(dest) = instr.destination() {
                    let _output = name_mapping.insert(dest, next_name);
                    debug_assert!(_output.is_none());
                    next_name += 1;
                }
            }
        }

        // Now remap all operands according to the mapping
        for bb in self.body.values_mut() {
            for instr in &mut bb.instructions {
                for op in instr.operands_mut() {
                    if let Operand::Reg(name) = op {
                        if let Some(new_name) = name_mapping.get(name) {
                            *name = *new_name;
                        } else {
                            unreachable!();
                        }
                    }
                }
            }
        }
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

//! Shared utilities and error types used across the hyinstr crate.
use strum::{EnumIs, EnumTryAs};
use thiserror::Error;
use uuid::Uuid;

use crate::modules::operand::{Label, Name};
#[cfg(feature = "chumsky")]
use crate::modules::symbol::FunctionPointerType;

#[cfg(feature = "chumsky")]
#[derive(Debug, Clone)]
/// Error detail returned by the optional text parser front-end.
pub struct ParserError {
    /// Path of the parsed file when available.
    pub file: Option<String>,
    /// Start byte offset of the offending span.
    pub start: usize,
    /// End byte offset (exclusive) of the offending span.
    pub end: usize,
    /// Human readable message describing the parse failure.
    pub message: String,
}

#[derive(Debug, EnumIs, EnumTryAs, Error)]
/// Generic error enumeration surfaced by hyinstr operations.
pub enum Error {
    /// An operand refers to a name that has not been defined.
    #[error(
        "Multiple operations with shared destination target violate SSA requirements. The name `{duplicate}` is defined more than once within the same function."
    )]
    DuplicateSSAName { duplicate: Name },

    /// No basic block with the entrypoint label was found.
    #[error(
        "By convention, the entrypoint basic block of a function must have label `label_0`. No such basic block was found."
    )]
    MissingEntryBlock,

    /// An operand refers to an unresolved name.
    #[error(
        "A operand refers to an undefined name: `{undefined}`. This name was never defined in the module."
    )]
    UndefinedSSAName { undefined: Name },

    /// Provided internal function is not defined within the module.
    #[error(
        "An instruction of function `{function}` refers to an internal function referenced by `{undefined}` that is not defined within the module."
    )]
    UndefinedInternalFunction { function: String, undefined: Uuid },

    /// Provided external function is not defined within the module.
    #[error(
        "An instruction of function `{function}` refers to an external function referenced by `{undefined}` that is not defined within the module."
    )]
    UndefinedExternalFunction { function: String, undefined: Uuid },

    /// Unsound wildcard type usage.
    #[error(
        "Unsound wildcard type usage in function `{function}`: expected wildcard types `{expected:?}`, but found `{found:?}`."
    )]
    UnsoundWildcardTypes {
        function: String,
        expected: Vec<String>,
        found: Vec<String>,
    },

    /// Meta operands are not allowed in this context.
    #[error(
        "Meta operands are only available internally for properties and attributes constructions. They SHOULD NOT appear in regular instructions."
    )]
    MetaOperandNotAllowed,

    /// Phi instructions must be the first instructions or following other phi instructions in a basic block.
    #[error(
        "Phi instructions must be the first instructions in a basic block or follow other phi instructions. The basic block `{block}` contains a phi instruction that is not the first instruction."
    )]
    PhiNotFirstInstruction { block: Label },

    /// The basic block referenced cannot be found within the function.
    #[error(
        "The basic block `{label}` referenced in function `{function}` is not defined within the function."
    )]
    UndefinedBasicBlock { function: String, label: Label },

    /// Meta instructions are not allowed in this context.
    #[error(
        "Meta instructions are only available internally for properties and attributes constructions. They SHOULD NOT appear in regular instructions. Function `{function}` contains a meta-instruction `{instruction}`."
    )]
    MetaInstructionNotAllowed {
        function: String,
        instruction: String,
    },

    /// Function exceeds maximum allowed number of basic blocks.
    #[error(
        "Function `{function}` contains {count} basic blocks, exceeding the maximum allowed of {max}."
    )]
    FunctionTooManyBlocks {
        function: String,
        count: usize,
        max: usize,
    },

    /// Basic block exceeds maximum allowed number of instructions.
    #[error(
        "Basic block `{block}` in function `{function}` contains {count} instructions, exceeding the maximum allowed of {max}."
    )]
    BasicBlockTooLarge {
        function: String,
        block: Label,
        count: usize,
        max: usize,
    },

    /// Function exceeds maximum allowed number of instructions.
    #[error(
        "Function `{function}` contains {count} instructions, exceeding the maximum allowed of {max}."
    )]
    FunctionTooManyInstructions {
        function: String,
        count: usize,
        max: usize,
    },

    /// Function exceeds maximum allowed number of parameters.
    #[error(
        "Function `{function}` contains {count} parameters, exceeding the maximum allowed of {max}."
    )]
    FunctionTooManyArguments {
        function: String,
        count: usize,
        max: usize,
    },

    /// Function exceeds maximum allowed number of wildcard types.
    #[error(
        "Function `{function}` contains {count} wildcard types, exceeding the maximum allowed of {max}."
    )]
    FunctionTooManyWildcardTypes {
        function: String,
        count: usize,
        max: usize,
    },

    /// A basic block with the given label already exists in the function.
    #[error("A basic block with label `{0}` already exists in the function.")]
    BlockLabelAlreadyExists(Label),

    /// The provided file does not exist or is not accessible.
    #[error("The provided file `{path}` does not exist or is not accessible: {cause}")]
    FileNotFound { path: String, cause: std::io::Error },

    #[cfg(feature = "chumsky")]
    #[error("Parser errors occurred: {errors:?}")]
    ParserErrors {
        errors: Vec<ParserError>,
        tokens: Vec<String>,
    },

    /// A function with the given name already exists in the module.
    #[cfg(feature = "chumsky")]
    #[error("A function with the name `{name}` already exists in the module.")]
    DuplicateFunctionName { name: String, file: String },

    /// Internal functions were referenced but not defined within the module.
    #[cfg(feature = "chumsky")]
    #[error(
        "The following internal functions were referenced but not defined within the module: {names:?}"
    )]
    UnresolvedInternalFunctions { names: Vec<String> },

    /// External functions were referenced but not defined within the module.:w
    #[error("A function with the name `{name}` already exists in the module.")]
    FunctionAlreadyExists { name: String },

    /// External or internal function was referenced but not defined within the module.
    #[cfg(feature = "chumsky")]
    #[error(
        "The {func_type} function `{name}` was referenced but not defined within the module as either an internal or external function."
    )]
    UnresolvedFunction {
        name: String,
        func_type: FunctionPointerType,
    },

    /// Type mismatch encountered during type checking.
    #[error(
        "Type mismatch in instruction `{instr}`: expected type `{expected}`, but found type `{found}`."
    )]
    TypeMismatch {
        instr: String,
        expected: String,
        found: String,
    },

    /// Type index is out of bounds.
    #[error(
        "The provided index `{index}` is out of bounds for type `{ty}` with maximum index `{max}` in instruction `{instr}`."
    )]
    ElementIndexOutOfBounds {
        ty: String,
        instr: String,
        index: usize,
        max: usize,
    },
}

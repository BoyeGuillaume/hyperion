use strum::{EnumIs, EnumTryAs};
use thiserror::Error;
use uuid::Uuid;

use crate::modules::operand::{Label, Name};

#[derive(Debug, PartialEq, Eq, Hash, EnumIs, EnumTryAs, Error)]
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
}

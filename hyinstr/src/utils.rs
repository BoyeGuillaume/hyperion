use strum::{EnumIs, EnumTryAs};
use thiserror::Error;

use crate::modules::operand::Name;

#[derive(Debug, PartialEq, Eq, Hash, EnumIs, EnumTryAs, Error)]
pub enum Error {
    /// An operand refers to a name that has not been defined.
    #[error(
        "Multiple operations with shared destination target violate SSA requirements. The name `{duplicate}` is defined more than once within the same function."
    )]
    DuplicateSSAName { duplicate: Name },

    /// No basic block with the entrypoint UUID was found.
    #[error(
        "By convention, the entrypoint basic block must have the nil UUID (all zeros), but no such block was found."
    )]
    MissingEntryBlock,

    /// An operand refers to an unresolved name.
    #[error(
        "A operand refers to an undefined name: `{undefined}`. This name was never defined in the module."
    )]
    UndefinedSSAName { undefined: Name },
}

use enum_map::Enum;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use strum::{EnumDiscriminants, EnumIs, EnumIter, EnumTryAs, IntoEnumIterator};

use crate::modules::instructions::InstructionFlags;
use crate::modules::operand::Label;

/// Possible termination behaviors of a block of instructions/function.
///
/// Of course, halting behavior is undecidable in the general case however we can
/// provide proof and analysis for a subset of cases.
///
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Enum)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum TerminationBehavior {
    /// Normal termination behavior.
    ///
    /// The program or function completes its execution without errors in finite time.
    Normal,

    /// Termination due to a trap or error.
    ///
    /// The program or function encounters an error condition that causes it to halt abnormally.
    Trap,

    /// Non-termination (divergence).
    ///
    /// The program or function enters an infinite loop or recursive calls without a base case,
    /// preventing it from reaching a normal termination.
    Diverge,
}

impl TerminationBehavior {
    /// Convert to integer representation.
    pub fn to_u8(self) -> u8 {
        match self {
            TerminationBehavior::Normal => 0,
            TerminationBehavior::Trap => 1,
            TerminationBehavior::Diverge => 2,
        }
    }

    /// Create from integer representation.
    pub fn from_u8(value: u8) -> Option<TerminationBehavior> {
        match value {
            0 => Some(TerminationBehavior::Normal),
            1 => Some(TerminationBehavior::Trap),
            2 => Some(TerminationBehavior::Diverge),
            _ => None,
        }
    }
}

/// Scope over which termination behavior is evaluated.
///
/// Selects the region of interest for the termination query.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub enum TerminationScope {
    /// Termination is defined as reaching the end of the current block.
    BlockExit,
    /// Termination of the entire function (any return/trap or completion of all paths).
    FunctionExit,
    /// Termination if any label in the set is reached.
    ReachAny(Vec<Label>),
}

/// Analysis statistics that can be used to gather information about behavior of
/// an block of instructions/function during execution or simulation.
///
#[derive(Debug, Clone, Hash, PartialEq, Eq, EnumIs, EnumTryAs, EnumDiscriminants)]
#[strum_discriminants(name(AnalysisStatisticOp))]
#[strum_discriminants(derive(EnumIter))]
#[cfg_attr(feature = "serde", strum_discriminants(derive(Serialize, Deserialize)))]
#[cfg_attr(
    feature = "borsh",
    strum_discriminants(derive(borsh::BorshSerialize, borsh::BorshDeserialize))
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub enum AnalysisStatistic {
    /// Count of instructions executed containing any of the specified flags.
    ///
    /// Note: number of instructions executed is per thread/per device.
    /// Note: Meta-instructions are not counted towards this statistic.
    /// Note: This is number of instructions since start up, not per function/block. Therefore
    ///       to get per function/block counts, the difference between two measurements (before/after)
    ///
    /// Example: e.g. [`InstructionFlags::MEMORY`] this will count how many
    /// memory instructions were executed.
    InstructionCount(InstructionFlags),

    /// Number of times this function was executed (useful for loop counts, recursion depth, etc).
    ExecutionCount,

    /// Termination behavior over a chosen scope. Returns an integer similar
    /// to [`TerminationBehavior::to_u8()`].
    ///
    /// The scope determines whether we consider the current block, the entire function,
    /// or reaching specified points in the CFG.
    TerminationBehavior(TerminationScope),
}

impl AnalysisStatistic {
    /// Get the operation type of this statistic.
    pub fn op(&self) -> AnalysisStatisticOp {
        self.into()
    }
}

impl AnalysisStatisticOp {
    /// Convert to string representation.
    pub fn to_str(&self) -> &'static str {
        match self {
            AnalysisStatisticOp::InstructionCount => "icnt",
            AnalysisStatisticOp::TerminationBehavior => "term",
            AnalysisStatisticOp::ExecutionCount => "excnt",
        }
    }
}

impl std::str::FromStr for AnalysisStatisticOp {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        AnalysisStatisticOp::iter()
            .find(|op| op.to_str() == s)
            .ok_or(())
    }
}

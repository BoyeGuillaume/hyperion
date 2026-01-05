use strum::{EnumDiscriminants, EnumIs, EnumIter, EnumTryAs, IntoEnumIterator};

use crate::modules::instructions::InstructionFlags;

/// Possible termination behaviors of a block of instructions/function.
///
/// Of course, halting behavior is undecidable in the general case however we can
/// provide proof and analysis for a subset of cases.
///
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
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

/// Analysis statistics that can be used to gather information about behavior of
/// an block of instructions/function during execution or simulation.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, EnumIs, EnumTryAs, EnumDiscriminants)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[strum_discriminants(name(AnalysisStatisticOp))]
#[strum_discriminants(derive(EnumIter))]
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

    /// Termination behavior observed at the given point. This returns an integer similar
    /// to [`TerminationBehavior::to_u8()`].
    ///
    /// Note: This is the termination/behavior of the current function/block. For instance the following
    /// ```llvm
    /// block_0:
    ///   %0: i2 = !statistic.termination_behavior
    ///   !assert.eq %0, i2 0                      ; assert normal termination, this asserts that the some_function below terminates normally
    ///   %1: i32 = invoke %some_function, %2, i32 0
    ///   !assert.eq %0, i2 0                      ; this doesn't do anything because either invoke succeeded but if trapped, or diverged, assert is not reached
    /// ```
    TerminationBehavior,

    /// Number of times this function was executed (useful for loop counts, recursion depth, etc).
    ///
    /// Note: Combine with `phi` nodes to assert loop iteration counts (outside the loop body).
    ExecutionCount,
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

    /// Parse from string representation.
    pub fn from_str(s: &str) -> Option<Self> {
        AnalysisStatisticOp::iter().find(|op| op.to_str() == s)
    }
}

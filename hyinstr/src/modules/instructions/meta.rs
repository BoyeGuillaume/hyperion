//! Meta-instructions used for analysis, specification, and probabilistic reasoning.
//!
//! These nodes never appear in executable code and are filtered out during
//! verification when `Function::meta_function` is false.
use crate::{
    analysis::AnalysisStatistic,
    modules::{
        instructions::{Instruction, InstructionFlags},
        operand::{Name, Operand},
    },
    types::Typeref,
};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use strum::{EnumDiscriminants, EnumIs, EnumIter, EnumTryAs, IntoEnumIterator};

/// Assertion instruction
///
/// This is a meta-instruction used for verification purposes. It should never
/// appear in executable code. It should point to a condition that must hold at
/// this program point. Therefore `assert %cond` signifies that `%cond` IS true
/// and is similar to the statement `%cond == true`. Proof for assertions can
/// be provided by the derivation engine or external tools.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct MetaAssert {
    /// The condition to assert. This should evaluate to a boolean value.
    pub condition: Operand,
}

impl Instruction for MetaAssert {
    fn flags(&self) -> InstructionFlags {
        InstructionFlags::META | InstructionFlags::SIMPLE
    }

    fn operands(&self) -> impl Iterator<Item = &Operand> {
        std::iter::once(&self.condition)
    }

    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        std::iter::once(&mut self.condition)
    }

    fn destination(&self) -> Option<Name> {
        None
    }

    fn referenced_types(&self) -> impl Iterator<Item = Typeref> {
        std::iter::empty()
    }

    fn referenced_types_mut(&mut self) -> impl Iterator<Item = &mut Typeref> {
        std::iter::empty()
    }

    fn destination_type(&self) -> Option<Typeref> {
        None
    }

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

/// Assumption instruction
///
/// This is a meta-instruction used to indicate that a certain condition is assumed
/// to hold at this program point. Unlike assertions, assumptions do not require proof
/// and are used to guide optimizations or analyses.
///
/// This is used in specifications as a 'precondition' check.
///
/// Basically, `assume %cond` signifies that all paths where `%cond` is false are impossible
/// while `assert %cond` signifies that ∵ `%cond` is true.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct MetaAssume {
    /// The condition to assume. This should evaluate to a boolean value.
    pub condition: Operand,
}

impl Instruction for MetaAssume {
    fn flags(&self) -> InstructionFlags {
        InstructionFlags::META | InstructionFlags::SIMPLE
    }

    fn operands(&self) -> impl Iterator<Item = &Operand> {
        std::iter::once(&self.condition)
    }

    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        std::iter::once(&mut self.condition)
    }

    fn destination(&self) -> Option<Name> {
        None
    }

    fn referenced_types(&self) -> impl Iterator<Item = Typeref> {
        std::iter::empty()
    }

    fn referenced_types_mut(&mut self) -> impl Iterator<Item = &mut Typeref> {
        std::iter::empty()
    }

    fn destination_type(&self) -> Option<Typeref> {
        None
    }

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

/// Check whether an operand is fully defined (no undef/poison content).
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct MetaIsDef {
    /// Destination SSA name holding the boolean result.
    pub dest: Name,
    /// Resulting type (should be a boolean integer).
    pub ty: Typeref,
    /// Operand to check for definedness.
    pub operand: Operand,
}

impl Instruction for MetaIsDef {
    fn flags(&self) -> InstructionFlags {
        InstructionFlags::META | InstructionFlags::SIMPLE
    }

    fn operands(&self) -> impl Iterator<Item = &Operand> {
        std::iter::once(&self.operand)
    }

    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        std::iter::once(&mut self.operand)
    }

    fn destination(&self) -> Option<Name> {
        Some(self.dest)
    }

    fn set_destination(&mut self, name: Name) {
        self.dest = name;
    }

    fn referenced_types(&self) -> impl Iterator<Item = Typeref> {
        std::iter::once(self.ty)
    }

    fn referenced_types_mut(&mut self) -> impl Iterator<Item = &mut Typeref> {
        std::iter::once(&mut self.ty)
    }

    fn destination_type(&self) -> Option<Typeref> {
        Some(self.ty)
    }

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

/// Create a universally quantified ghost value.
///
/// This meta-instruction introduces a side-effect-free, unbound SSA value of a
/// given type to support quantified reasoning (∀). It produces a destination
/// register of the requested type and has no operands.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct MetaForall {
    /// Destination SSA name holding the quantified value.
    pub dest: Name,
    /// The type of the quantified value.
    pub ty: Typeref,
}

impl Instruction for MetaForall {
    fn flags(&self) -> InstructionFlags {
        InstructionFlags::META
    }

    fn operands(&self) -> impl Iterator<Item = &Operand> {
        std::iter::empty()
    }

    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        std::iter::empty()
    }

    fn destination(&self) -> Option<Name> {
        Some(self.dest)
    }

    fn referenced_types_mut(&mut self) -> impl Iterator<Item = &mut Typeref> {
        std::iter::once(&mut self.ty)
    }

    fn set_destination(&mut self, name: Name) {
        self.dest = name;
    }

    fn referenced_types(&self) -> impl Iterator<Item = Typeref> {
        std::iter::once(self.ty)
    }

    fn destination_type(&self) -> Option<Typeref> {
        Some(self.ty)
    }
}

/// Probability operand types
///
/// Models different kinds of probability-related operands that can be used in
/// probabilistic programming constructs.
#[derive(Debug, Clone, Hash, PartialEq, Eq, EnumIs, EnumTryAs, EnumDiscriminants)]
#[strum_discriminants(name(MetaProbVariant))]
#[strum_discriminants(derive(EnumIter))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub enum MetaProbOperand {
    /// Operand (boolean) value, this is the probability that the given operand is true. Input
    /// should be a boolean value.
    Probability(Operand),

    /// Expected value of an operand
    ///
    /// Operand should be a numeric value (integer or floating-point), this represents the expected
    /// value of that operand in average defined as E[X] = Σ [ x * P(X=x) ] over all possible values x.
    ExpectedValue(Operand),

    /// Variance of an operand
    ///
    /// Operand should be a numeric value (integer or floating-point), this represents the variance
    /// of that operand defined as Var(X) = E[(X - E[X])^2].
    Variance(Operand),
}

impl MetaProbVariant {
    /// Get the arity of the probability operand
    ///
    /// Unlike other instructions, (which have the [`super::instructions::HyInstrOp::arity`] method),
    /// the arity of a [`MetaProb`] instruction depends on the specific variant of the
    /// [`MetaProbOperand`]. This method returns the number of operands required for the
    /// specific probability operation.
    pub fn arity(&self) -> usize {
        match self {
            MetaProbVariant::Probability
            | MetaProbVariant::ExpectedValue
            | MetaProbVariant::Variance => 1,
        }
    }

    /// Get name of the variant as a string
    pub fn to_str(&self) -> &'static str {
        match self {
            MetaProbVariant::Probability => "prb",   /* boolean */
            MetaProbVariant::ExpectedValue => "xpt", /* numeric */
            MetaProbVariant::Variance => "var",      /* numeric */
        }
    }
}

impl std::str::FromStr for MetaProbVariant {
    type Err = ();

    fn from_str(s: &str) -> Result<MetaProbVariant, Self::Err> {
        MetaProbVariant::iter()
            .find(|variant| variant.to_str() == s)
            .ok_or(())
    }
}

/// Probability function instruction
///
/// This operation represents a probability function used in probabilistic programming.
/// It is a meta-instruction and should not appear in executable code. It is used
/// for modeling and reasoning about probabilistic computations.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct MetaProb {
    /// The destination SSA name for the result of the probability function.
    pub dest: Name,

    /// The output type of the probability function.
    ///
    /// This should always be a floating-point type representing a probability value
    pub ty: Typeref,

    /// The operand representing the probability input.
    pub operand: MetaProbOperand,
}

impl Instruction for MetaProb {
    fn flags(&self) -> InstructionFlags {
        InstructionFlags::META | InstructionFlags::SIMPLE
    }

    fn operands(&self) -> impl Iterator<Item = &Operand> {
        match &self.operand {
            MetaProbOperand::Probability(op)
            | MetaProbOperand::ExpectedValue(op)
            | MetaProbOperand::Variance(op) => std::iter::once(op),
        }
    }

    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        match &mut self.operand {
            MetaProbOperand::Probability(op)
            | MetaProbOperand::ExpectedValue(op)
            | MetaProbOperand::Variance(op) => std::iter::once(op),
        }
    }

    fn destination(&self) -> Option<Name> {
        Some(self.dest)
    }

    fn set_destination(&mut self, name: Name) {
        self.dest = name;
    }

    fn referenced_types(&self) -> impl Iterator<Item = Typeref> {
        std::iter::once(self.ty)
    }

    fn referenced_types_mut(&mut self) -> impl Iterator<Item = &mut Typeref> {
        std::iter::once(&mut self.ty)
    }

    fn destination_type(&self) -> Option<Typeref> {
        Some(self.ty)
    }
}

/// Instruction used to query analysis statistics during execution or simulation.
///
/// Note: Some of those analysis statistics are tied to some architectural features or
/// hardware capabilities (e.g. performance counters, etc). Therefore, any statement/proof
/// containing those analysis must be tied to a specific architecture or hardware model.
///
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct MetaAnalysisStat {
    /// The destination SSA name for the result of the analysis statistic instruction.
    pub dest: Name,
    /// The output type of the analysis statistic.
    /// This should always be an integer type representing a count or measurement.
    pub ty: Typeref,
    /// The analysis statistic to query.
    pub statistic: AnalysisStatistic,
}

impl Instruction for MetaAnalysisStat {
    fn flags(&self) -> InstructionFlags {
        InstructionFlags::META | InstructionFlags::SIMPLE
    }

    fn operands(&self) -> impl Iterator<Item = &Operand> {
        std::iter::empty()
    }

    fn operands_mut(&mut self) -> impl Iterator<Item = &mut Operand> {
        std::iter::empty()
    }

    fn destination(&self) -> Option<Name> {
        Some(self.dest)
    }

    fn set_destination(&mut self, name: Name) {
        self.dest = name;
    }

    fn referenced_types(&self) -> impl Iterator<Item = Typeref> {
        std::iter::once(self.ty)
    }

    fn referenced_types_mut(&mut self) -> impl Iterator<Item = &mut Typeref> {
        std::iter::once(&mut self.ty)
    }

    fn destination_type(&self) -> Option<Typeref> {
        Some(self.ty)
    }
}

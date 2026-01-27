use std::sync::Weak;

use crate::{
    base::InstanceContext,
    formal::{DerivationStrategy, DynDerivationStrategy},
    register,
    utils::opaque::{OpaqueList, OpaqueObject},
};

#[derive(Debug)]
pub struct AxiomStrategyConfig {}
impl OpaqueObject for AxiomStrategyConfig {}

pub struct AxiomStrategy {
    instance: Weak<InstanceContext>,
}
register!(derivation_strategy AxiomStrategy);

impl DerivationStrategy for AxiomStrategy {
    const NAME: &'static str = "AxiomStrategy";

    fn new(
        instance: Weak<InstanceContext>,
        _ext: &mut OpaqueList,
    ) -> crate::utils::error::HyResult<Self> {
        Ok(Self { instance })
    }
}

impl DynDerivationStrategy for AxiomStrategy {
    fn instance(&self) -> &Weak<InstanceContext> {
        &self.instance
    }

    fn derive(&self) {
        // Implementation of derivation logic goes here.
    }
}

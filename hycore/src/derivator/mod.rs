use std::usize;

use hyinstr::modules::Function;
use strum::{EnumIs, EnumTryAs};

use crate::attributes::function::FunctionMetadata;

/// Context for property extraction and derivation.
pub struct ExtractorContext<'a, C> {
    /// Metadata about the function being analyzed.
    pub func_metadata: &'a mut FunctionMetadata,
    /// The function being analyzed.
    pub function: &'a Function,
    /// Context specific to the derivator. Useful for restarting from previous states.
    pub context: C,
}

/// Information about an interrupted property derivation.
#[derive(Debug, Clone)]
pub struct PropDerivatorRunInfo {
    pub iteration: usize,
    pub time_elapsed: std::time::Duration,
}

/// Enumeration of possible steps after a property derivation step.
#[derive(Debug, Clone, EnumIs, EnumTryAs)]
pub enum PropDerivatorStatus {
    Continue,
    Terminated,
    Error(String),
}

pub trait PropDerivator {
    type Context;

    fn initialize_context(&self) -> Self::Context;

    fn derive_props_step(
        &self,
        context: &'_ mut ExtractorContext<'_, Self::Context>,
    ) -> PropDerivatorStatus;

    fn derive_props_finalize(&self, context: ExtractorContext<'_, Self::Context>);

    fn derive_props(
        &self,
        func_metadata: &'_ mut FunctionMetadata,
        func: &'_ Function,
        context: Option<Self::Context>,
        iterator_budget: Option<usize>,
        time_budget: Option<std::time::Duration>,
        finalize_on_termination: bool,
    ) -> (
        Option<Self::Context>,
        PropDerivatorStatus,
        PropDerivatorRunInfo,
    ) {
        let mut context = ExtractorContext {
            func_metadata,
            function: func,
            context: context.unwrap_or_else(|| self.initialize_context()),
        };

        let start_time = std::time::Instant::now();
        let mut budget = iterator_budget.unwrap_or(usize::MAX);
        let (out_state, out_run_info) = loop {
            let run_info = PropDerivatorRunInfo {
                iteration: iterator_budget.unwrap_or(usize::MAX) - budget,
                time_elapsed: start_time.elapsed(),
            };

            if let Some(time_budget) = time_budget {
                if run_info.time_elapsed >= time_budget {
                    break (PropDerivatorStatus::Continue, run_info);
                }
            }

            if budget == 0 {
                break (PropDerivatorStatus::Continue, run_info);
            }

            match self.derive_props_step(&mut context) {
                PropDerivatorStatus::Continue => {
                    budget -= 1;
                }
                v => {
                    break (v, run_info);
                }
            }
        };

        if finalize_on_termination
            || matches!(
                out_state,
                PropDerivatorStatus::Terminated | PropDerivatorStatus::Error(_)
            )
        {
            self.derive_props_finalize(context);
            (None, out_state, out_run_info)
        } else {
            (Some(context.context), out_state, out_run_info)
        }
    }
}

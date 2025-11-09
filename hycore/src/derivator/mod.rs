//! Property derivation (extraction) runtime.
//!
//! This module defines the traits and helper structures used to implement
//! iterative derivation of properties for functions. A "derivator" is an
//! algorithm that repeatedly refines a property set until a termination
//! condition (fixed point, resource exhaustion, explicit success) is reached.
//!
//! ## Core concepts
//! * [`PropDerivator`] – Trait implemented by concrete derivation algorithms.
//! * [`ExtractorContext`] – Mutable handle passed on each step containing
//!   function metadata, analysis context, and derivator-specific state.
//! * [`PropDerivatorRunArguments`] – Inputs controlling a derivation session
//!   (iteration/time budgets, resume context, finalization semantics).
//! * [`PropDerivatorRunResult`] – Outputs including status, run info, and
//!   optionally a serializable context for resumption.
//! * [`DynPropDerivator`] – Object-safe dispatcher enabling heterogeneous derivators.
//!
//! ## Typical usage
//! ```no_run
//! # use hycore::derivator::{PropDerivator, PropDerivatorRunArguments};
//! # struct MyDerivator; impl PropDerivator for MyDerivator {
//! #   type Context = ();
//! #
//! #   fn initialize_context(&self)-> Self::Context {}
//! #   
//! #   fn derive_props_step(&self, _:&mut hycore::derivator::ExtractorContext<'_, Self::Context>) -> hycore::derivator::PropDerivatorStatus {
//! #      hycore::derivator::PropDerivatorStatus::Terminated
//! #   }
//! #  
//! #   fn derive_props_finalize(&self,_:hycore::derivator::ExtractorContext<'_, Self::Context>)
//! #   {}
//! # }
//! #
//! # use hyinstr::modules::FunctionAnalysisContext;
//! # let fac: FunctionAnalysisContext<'_> = unsafe { std::mem::zeroed() }; // placeholder
//! # use hycore::axioms::function::FunctionMetadata; let mut fm = FunctionMetadata { properties: vec![] };
//! let derivator = MyDerivator;
//! let result = derivator.derive_props(PropDerivatorRunArguments {
//!     iteration_budget: Some(100),
//!     time_budget: None,
//!     func_metadata: &mut fm,
//!     function_analysis: &fac,
//!     context: None,
//!     finalize_on_termination: true,
//! });
//! assert!(result.status.is_terminated());
//! ```
//!
//! ## Budget semantics
//! * `iteration_budget`: Maximum number of successful `Continue` steps; `usize::MAX` if `None`.
//! * `time_budget`: Hard wall-clock cutoff; expiration returns `Continue` with captured run info.
//! * A derivator returning `Terminated` or `Error` triggers finalize if requested.
//!
//! ## Object safety
//! The blanket impl for `DynPropDerivator` enables storing heterogeneous derivators
//! behind trait objects while still supporting resumable contexts via `Box<dyn Any>`.
use std::usize;

use hyinstr::modules::FunctionAnalysisContext;
use strum::{EnumIs, EnumTryAs};

use crate::axioms::function::FunctionMetadata;

/// Mutable context passed to each derivation step.
///
/// Contains stable references to function metadata and analysis structures plus
/// derivator-specific state `C` (which can be persisted across sessions).
pub struct ExtractorContext<'a, C> {
    /// Accumulated metadata for the function being analyzed.
    pub func_metadata: &'a mut FunctionMetadata,
    /// Analysis context.
    pub function_analysis: &'a FunctionAnalysisContext<'a>,
    /// Context specific to the derivator. Useful for restarting from previous states.
    pub context: C,
}

/// Outcome of a single derivation step or terminal result.
#[derive(Debug, Clone, EnumIs, EnumTryAs)]
pub enum PropDerivatorStatus {
    Continue,
    Terminated,
    Error(String),
}

/// Snapshot metrics about a (possibly interrupted) derivation run.
#[derive(Debug, Clone)]
pub struct PropDerivatorRunInfo {
    pub iteration: usize,
    pub time_elapsed: std::time::Duration,
}

/// Result bundle produced by a derivation session.
///
/// If `context` is `Some`, the derivator can resume from that state; otherwise
/// the derivation finalized and released its internal state.
#[derive(Debug, Clone)]
pub struct PropDerivatorRunResult<C> {
    pub context: Option<C>,
    pub status: PropDerivatorStatus,
    pub run_info: PropDerivatorRunInfo,
}

/// Input configuration for a derivation session.
pub struct PropDerivatorRunArguments<'a, C> {
    pub iteration_budget: Option<usize>,
    pub time_budget: Option<std::time::Duration>,
    pub func_metadata: &'a mut FunctionMetadata,
    pub function_analysis: &'a FunctionAnalysisContext<'a>,
    pub context: Option<C>,
    pub finalize_on_termination: bool,
}

/// Trait implemented by concrete property derivation algorithms.
///
/// ### Required methods
/// * `initialize_context` – Produce an initial context state (often empty or seeded).
/// * `derive_props_step` – Perform one refinement step. Return `Continue` to keep going,
///   or `Terminated` / `Error` to stop.
/// * `derive_props_finalize` – Consume the context and produce any final materializations.
///
/// ### Provided method
/// * `derive_props` – Orchestrates timed / budgeted stepping, finalization, and resumption.
pub trait PropDerivator {
    type Context;

    /// Create an initial context state for a fresh derivation session.
    fn initialize_context(&self) -> Self::Context;

    /// Execute a single derivation step mutating the passed context.
    fn derive_props_step(
        &self,
        context: &'_ mut ExtractorContext<'_, Self::Context>,
    ) -> PropDerivatorStatus;

    /// Finalize a derivation (flush derived properties, normalize invariants, etc.).
    fn derive_props_finalize(&self, context: ExtractorContext<'_, Self::Context>);

    /// Run a derivation session with optional iteration/time budgets and resumable context.
    fn derive_props(
        &self,
        argument: PropDerivatorRunArguments<'_, Self::Context>,
    ) -> PropDerivatorRunResult<Self::Context> {
        let mut context = ExtractorContext {
            func_metadata: argument.func_metadata,
            function_analysis: argument.function_analysis,
            context: argument
                .context
                .unwrap_or_else(|| self.initialize_context()),
        };

        let start_time = std::time::Instant::now();
        let mut budget = argument.iteration_budget.unwrap_or(usize::MAX);
        let (out_state, out_run_info) = loop {
            let run_info = PropDerivatorRunInfo {
                iteration: argument.iteration_budget.unwrap_or(usize::MAX) - budget,
                time_elapsed: start_time.elapsed(),
            };

            if let Some(time_budget) = argument.time_budget {
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

        if argument.finalize_on_termination
            || matches!(
                out_state,
                PropDerivatorStatus::Terminated | PropDerivatorStatus::Error(_)
            )
        {
            self.derive_props_finalize(context);
            PropDerivatorRunResult {
                context: None,
                status: out_state,
                run_info: out_run_info,
            }
        } else {
            PropDerivatorRunResult {
                context: Some(context.context),
                status: out_state,
                run_info: out_run_info,
            }
        }
    }
}

impl<T: PropDerivator> DynPropDerivator for T
where
    T::Context: 'static,
{
    fn derive_props_any(
        &self,
        argument: PropDerivatorRunArguments<'_, Box<dyn std::any::Any>>,
    ) -> PropDerivatorRunResult<Box<dyn std::any::Any>> {
        let context = argument
            .context
            .map(|c| *c.downcast::<T::Context>().expect("Invalid context type"));

        let result = self.derive_props(PropDerivatorRunArguments {
            iteration_budget: argument.iteration_budget,
            time_budget: argument.time_budget,
            func_metadata: argument.func_metadata,
            function_analysis: argument.function_analysis,
            context,
            finalize_on_termination: argument.finalize_on_termination,
        });

        PropDerivatorRunResult {
            context: result
                .context
                .map(|c| Box::new(c) as Box<dyn std::any::Any>),
            status: result.status,
            run_info: result.run_info,
        }
    }
}

/// Object-safe derivator interface enabling heterogeneous scheduling.
pub trait DynPropDerivator {
    fn derive_props_any(
        &self,
        argument: PropDerivatorRunArguments<'_, Box<dyn std::any::Any>>,
    ) -> PropDerivatorRunResult<Box<dyn std::any::Any>>;
}

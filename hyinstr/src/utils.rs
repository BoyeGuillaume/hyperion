//! Shared utilities and error types used across the hyinstr crate.
use strum::{EnumIs, EnumTryAs};
use thiserror::Error;

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
    /// Input file missing.
    #[error("File not found: {0}")]
    FileNotFound(String),

    /// Limits exceeded for logical objects (blocks, instructions, etc.).
    #[error("Too many {object}: {details} (limit: {limit})")]
    TooManyObjects {
        object: &'static str,
        limit: usize,
        details: String,
    },

    /// Generic validation failure when IR invariants are broken.
    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    /// Invalid arguments supplied by callers.
    #[error("Illegal argument: {0}")]
    IllegalArgument(String),

    /// Illegal state encountered.
    #[error("Illegal state: {0}")]
    IllegalState(String),

    /// Operation forbidden in the current context.
    #[error("Operation not permitted: {0}")]
    OperationNotPermitted(String),

    /// Type mismatch encountered during type checking.
    #[error(
        "Type mismatch in instruction `{instr}`: expected type `{expected}`, but found type `{found}`."
    )]
    TypeMismatch {
        instr: String,
        expected: String,
        found: String,
    },

    #[cfg(feature = "chumsky")]
    #[error("Parser errors occurred: {errors:?}")]
    ParserError {
        errors: Vec<ParserError>,
        tokens: Vec<String>,
    },
}

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum HyError {}

pub type HyResult<T> = Result<T, HyError>;

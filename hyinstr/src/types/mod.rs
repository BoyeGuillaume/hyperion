use uuid::Uuid;
pub mod primary;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Typeref(Uuid);



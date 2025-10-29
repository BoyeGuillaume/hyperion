#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum Name {
    // With Box to reduce size of enum variants
    Name(Box<String>),

    /// Doesn't have a string name and was given a number
    Number(usize),
}

impl From<String> for Name {
    fn from(s: String) -> Self {
        Name::Name(Box::new(s))
    }
}

impl From<&str> for Name {
    fn from(s: &str) -> Self {
        Name::Name(Box::new(s.to_string()))
    }
}

impl From<usize> for Name {
    fn from(n: usize) -> Self {
        Name::Number(n)
    }
}

impl std::fmt::Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Name::Name(s) => write!(f, "{}", s),
            Name::Number(n) => write!(f, "%{}", n),
        }
    }
}

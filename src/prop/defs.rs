use crate::prop::{Prop, prop_sealed};

/// Represents a true proposition.
///
/// An atomic proposition that is always true.
///
pub struct PropTrue {}
impl prop_sealed::Sealed for PropTrue {}
impl Prop for PropTrue {}

/// Represents a false proposition.
///
/// An atomic proposition that is always false.
///
pub struct PropFalse {}
impl prop_sealed::Sealed for PropFalse {}
impl Prop for PropFalse {}

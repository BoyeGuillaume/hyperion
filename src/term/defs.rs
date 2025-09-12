use crate::{
    prop::Prop,
    term::{Term, term_sealed},
    variable::InlineVariable,
};

/// A dynamic term that holds whenever a term is deemed unreachable.
pub struct Unreachable;

impl term_sealed::Sealed for Unreachable {}
impl Term for Unreachable {}

/// Represents the application of a function to an argument.
///
/// If `f` is a variable representing a function and `A` is a term representing an argument,
/// then `App<A>` represents the term `f(A)`.
pub struct App<A: Term> {
    pub func: InlineVariable,
    pub arg: A,
}

impl<A: Term> term_sealed::Sealed for App<A> {}
impl<A: Term> Term for App<A> {}

/// Represents a variable term.
///
/// A variable term is simply a reference to a variable identified by its name.
impl term_sealed::Sealed for InlineVariable {}
impl Term for InlineVariable {}

/// Represents a conditional term.
///
/// If `P` is a proposition, `T` and `E` are terms, then `If<P, T, E>` represents the term
/// that evaluates to `T` if `P` is true, and `E` otherwise.
pub struct If<P: Prop, T: Term, E: Term> {
    pub condition: P,
    pub then_branch: T,
    pub else_branch: E,
}

impl<P: Prop, T: Term, E: Term> term_sealed::Sealed for If<P, T, E> {}
impl<P: Prop, T: Term, E: Term> Term for If<P, T, E> {}

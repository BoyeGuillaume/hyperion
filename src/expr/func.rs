// use crate::{
//     expr::{Expr, defs},
//     variable::InlineVariable,
// };

// pub fn forall<Q: Expr, R: Expr>(var: InlineVariable, dtype: Q, inner: R) -> defs::ForAll<Q, R> {
//     defs::ForAll {
//         variable: var,
//         dtype,
//         inner,
//     }
// }

// pub fn exists<Q: Expr, R: Expr>(var: InlineVariable, dtype: Q, inner: R) -> defs::Exists<Q, R> {
//     defs::Exists {
//         variable: var,
//         dtype,
//         inner,
//     }
// }

// pub fn tuple<A: Expr, B: Expr>(a: A, b: B) -> defs::Tuple<A, B> {
//     defs::Tuple {
//         first: a,
//         second: b,
//     }
// }

// pub fn func<A: Expr, B: Expr>(domain: A, codomain: B) -> defs::Func<A, B> {
//     defs::Func { domain, codomain }
// }

// pub fn branch<A: Expr, B: Expr, C: Expr>(
//     cond: A,
//     then_branch: B,
//     else_branch: C,
// ) -> defs::If<A, B, C> {
//     defs::If {
//         condition: cond,
//         then_branch,
//         else_branch,
//     }
// }

// pub fn powerset<A: Expr>(inner: A) -> defs::PowerSet<A> {
//     defs::PowerSet { inner }
// }

// pub fn equals<A: Expr, B: Expr>(left: A, right: B) -> defs::Eq<A, B> {
//     defs::Eq { left, right }
// }

// pub fn and<A: Expr, B: Expr>(left: A, right: B) -> defs::And<A, B> {
//     defs::And { left, right }
// }

// pub fn or<A: Expr, B: Expr>(left: A, right: B) -> defs::Or<A, B> {
//     defs::Or { left, right }
// }

// pub fn implies<A: Expr, B: Expr>(antecedent: A, consequent: B) -> defs::Implies<A, B> {
//     defs::Implies {
//         antecedent,
//         consequent,
//     }
// }

// pub fn iff<A: Expr, B: Expr>(left: A, right: B) -> defs::Iff<A, B> {
//     defs::Iff { left, right }
// }

// pub fn not<A: Expr>(inner: A) -> defs::Not<A> {
//     defs::Not { inner }
// }

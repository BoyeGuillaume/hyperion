//! Iterative, zero-copy walkers over encoded expressions.
//!
//! This module provides two traversal helpers that operate over [`AnyExprRef`](crate::expr::AnyExprRef)
//! without allocating new nodes or cloning the underlying buffer:
//! - [`walk`]: pass an initial input of any type to your visitor; each node can schedule which
//!   children to visit next with their own input values.
//! - [`walk_no_input`]: a convenience wrapper when you don't need to thread user state.
//!
//! Traversal strategy
//! - Adaptive iteration using an explicit stack (no recursion).
//! - You are in control: only children for which you call [`WalkerNodeHandle::schedule_visit`] are
//!   traversed. This makes partial traversals, pruning, and guided searches easy.
//!
//! Performance and memory footprint
//! - Time: O(n) nodes visited; per node we do a binary-search insertion over at most k children
//!   (k ≤ TreeBuf::MAX_NUM_REFERENCES, currently ≤ 3 for this language), so O(k log k) ≈ O(1).
//! - Memory: O(depth) for an internal `Vec` stack; a single `RefCell<StaticVec<...>>` buffer of
//!   capacity k is reused for child scheduling at each step (no heap allocations for that buffer).
//! - Allocations: visiting itself performs no heap allocations beyond the stack growth; all node
//!   views are borrowed and decoded on the fly.
//!
//! Example: count nodes in an expression
//! ```
//! use hyformal::expr::*;
//! use hyformal::expr::defs::{True, False};
//! use hyformal::walker::{walk_no_input, WalkerHandle};
//!
//! let expr = (True & !False).encode();
//! let mut count = 0usize;
//! walk_no_input(expr.as_ref(), |node| {
//!     // Always visit all children
//!     match &*node {
//!         view::ExprView::Not(child)
//!         | view::ExprView::Powerset(child) => {
//!             child.schedule_visit(())
//!         }
//!         view::ExprView::And(l, r)
//!         | view::ExprView::Or(l, r)
//!         | view::ExprView::Implies(l, r)
//!         | view::ExprView::Iff(l, r)
//!         | view::ExprView::Equal(l, r)
//!         | view::ExprView::Tuple(l, r)
//!         | view::ExprView::Lambda { arg: l, body: r }
//!         | view::ExprView::Call { func: l, arg: r }
//!         | view::ExprView::Forall { dtype: l, inner: r, .. }
//!         | view::ExprView::Exists { dtype: l, inner: r, .. } => {
//!             l.schedule_visit(());
//!             r.schedule_visit(());
//!         }
//!         view::ExprView::If { condition, then_branch, else_branch } => {
//!             condition.schedule_visit(());
//!             then_branch.schedule_visit(());
//!             else_branch.schedule_visit(());
//!         }
//!         _ => {}
//!     }
//!     count += 1;
//! });
//! assert_eq!(count, 4); // And(True, Not(False)) has 4 nodes
//! ```
//!
//! Example: guided search with state
//! ```
//! use hyformal::expr::*;
//! use hyformal::expr::defs::{True, Bool};
//! use hyformal::walker::walk;
//! use hyformal::expr::variant::ExprType;
//!
//! // Search for the first occurrence of a Tuple node, keep a tiny state with remaining budget
//! let expr = Bool.lambda(Bool).tuple(True).encode();
//! let mut found_tuple = false;
//! walk(expr.as_ref(), /*budget:*/ 100u32, |mut budget, node| {
//!     if budget == 0 { return; }
//!     budget -= 1;
//!     if node.type_() == ExprType::Tuple {
//!         found_tuple = true;
//!         return; // early stop: don't schedule more children
//!     }
//!     // Otherwise keep exploring all children with the updated budget
//!     match &*node {
//!         view::ExprView::Not(c) | view::ExprView::Powerset(c) => c.schedule_visit(budget),
//!         view::ExprView::And(l, r)
//!         | view::ExprView::Or(l, r)
//!         | view::ExprView::Implies(l, r)
//!         | view::ExprView::Iff(l, r)
//!         | view::ExprView::Equal(l, r)
//!         | view::ExprView::Tuple(l, r)
//!         | view::ExprView::Lambda { arg: l, body: r }
//!         | view::ExprView::Call { func: l, arg: r }
//!         | view::ExprView::Forall { dtype: l, inner: r, .. }
//!         | view::ExprView::Exists { dtype: l, inner: r, .. } => { l.schedule_visit(budget); r.schedule_visit(budget); }
//!         view::ExprView::If { condition, then_branch, else_branch } => {
//!             condition.schedule_visit(budget);
//!             then_branch.schedule_visit(budget);
//!             else_branch.schedule_visit(budget);
//!         }
//!         _ => {}
//!     }
//! });
//! assert!(found_tuple);
//! ```
mod immutable;
mod internal;
// mod mutable;

pub use immutable::*;
// pub use mutable::*;

use crate::expr::{Expr, view::ExprView};

/// Compare two expressions for structural equality.
///
/// Notice that this is different from the default [`PartialEq`] implementation
/// for [`AnyExpr`] and [`AnyExprRef`], which compare by identity (same buffer and node).
///
/// When comparing two AnyExpr, prefer using [`AnyExpr::eq`] or [`AnyExprRef::eq`] (default == operator)
/// over this function for better performance (early out on buffer/node mismatch).
///
pub fn compare_expressions<E1: Expr, E2: Expr>(a: E1, b: E2) -> bool {
    let a_view = a.view();
    let b_view = b.view();

    if a_view.type_() != b_view.type_() {
        return false;
    }

    match (a_view, b_view) {
        (ExprView::Bool, ExprView::Bool) => true,
        (ExprView::Omega, ExprView::Omega) => true,
        (ExprView::True, ExprView::True) => true,
        (ExprView::False, ExprView::False) => true,
        (ExprView::Never, ExprView::Never) => true,
        (ExprView::Not(inner_a), ExprView::Not(inner_b)) => compare_expressions(inner_a, inner_b),
        (ExprView::Powerset(inner_a), ExprView::Powerset(inner_b)) => {
            compare_expressions(inner_a, inner_b)
        }
        (ExprView::And(lhs_a, rhs_a), ExprView::And(lhs_b, rhs_b)) => {
            compare_expressions(lhs_a, lhs_b) && compare_expressions(rhs_a, rhs_b)
        }
        (ExprView::Or(lhs_a, rhs_a), ExprView::Or(lhs_b, rhs_b)) => {
            compare_expressions(lhs_a, lhs_b) && compare_expressions(rhs_a, rhs_b)
        }
        (ExprView::Implies(lhs_a, rhs_a), ExprView::Implies(lhs_b, rhs_b)) => {
            compare_expressions(lhs_a, lhs_b) && compare_expressions(rhs_a, rhs_b)
        }
        (ExprView::Iff(lhs_a, rhs_a), ExprView::Iff(lhs_b, rhs_b)) => {
            compare_expressions(lhs_a, lhs_b) && compare_expressions(rhs_a, rhs_b)
        }
        (ExprView::Equal(lhs_a, rhs_a), ExprView::Equal(lhs_b, rhs_b)) => {
            compare_expressions(lhs_a, lhs_b) && compare_expressions(rhs_a, rhs_b)
        }
        (
            ExprView::Lambda {
                arg: arg_a,
                body: body_a,
            },
            ExprView::Lambda {
                arg: arg_b,
                body: body_b,
            },
        ) => compare_expressions(arg_a, arg_b) && compare_expressions(body_a, body_b),
        (
            ExprView::Call {
                func: func_a,
                arg: arg_a,
            },
            ExprView::Call {
                func: func_b,
                arg: arg_b,
            },
        ) => compare_expressions(func_a, func_b) && compare_expressions(arg_a, arg_b),
        (ExprView::Tuple(lhs_a, rhs_a), ExprView::Tuple(lhs_b, rhs_b)) => {
            compare_expressions(lhs_a, lhs_b) && compare_expressions(rhs_a, rhs_b)
        }
        (
            ExprView::Forall {
                variable: variable_a,
                dtype: dtype_a,
                inner: inner_a,
            },
            ExprView::Forall {
                variable: variable_b,
                dtype: dtype_b,
                inner: inner_b,
            },
        ) => {
            variable_a == variable_b
                && compare_expressions(dtype_a, dtype_b)
                && compare_expressions(inner_a, inner_b)
        }
        (
            ExprView::Exists {
                variable: variable_a,
                dtype: dtype_a,
                inner: inner_a,
            },
            ExprView::Exists {
                variable: variable_b,
                dtype: dtype_b,
                inner: inner_b,
            },
        ) => {
            variable_a == variable_b
                && compare_expressions(dtype_a, dtype_b)
                && compare_expressions(inner_a, inner_b)
        }
        (
            ExprView::If {
                condition: condition_a,
                then_branch: then_branch_a,
                else_branch: else_branch_a,
            },
            ExprView::If {
                condition: condition_b,
                then_branch: then_branch_b,
                else_branch: else_branch_b,
            },
        ) => {
            compare_expressions(condition_a, condition_b)
                && compare_expressions(then_branch_a, then_branch_b)
                && compare_expressions(else_branch_a, else_branch_b)
        }
        (ExprView::Variable(inner_a), ExprView::Variable(inner_b)) => inner_a == inner_b,
        _ => unreachable!(),
    }
}

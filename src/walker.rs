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
//!     match node {
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
//!     match node {
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

use std::{cell::RefCell, collections::VecDeque, ops::Deref};

use either::Either;

use crate::{
    encoding::{
        EncodableExpr,
        tree::{TreeBuf, TreeBufNodeRef},
    },
    expr::{AnyExpr, AnyExprRef, Expr, view::ExprView},
    prelude::{False, True},
    utils::staticvec::StaticVec,
};

/// Lightweight handle passed to the visitor, representing a child node plus scheduling control.
///
/// - `E` is the underlying expression reference type (here [`AnyExprRef`]).
/// - `I` is the user-defined input/state type threaded through the traversal.
///
/// You can deref or `as_ref()` this handle to access the underlying expression, or call
/// [`schedule_visit`](Self::schedule_visit) to enqueue this child for a later DFS visit with an
/// input of your choice.
pub struct WalkerNodeHandle<'a, E, I> {
    stack: &'a RefCell<VecDeque<(TreeBufNodeRef, TreeBufNodeRef, I)>>,
    elem: E,
    children_node: TreeBufNodeRef,
    current_node: TreeBufNodeRef,
}

impl<'a, E, I> WalkerNodeHandle<'a, E, I> {
    /// Schedule this child to be visited immediately (LIFO), i.e., depth-first.
    /// Useful for drilling down before exploring siblings.
    #[inline]
    pub fn schedule_immediate(&self, input: I) {
        self.stack
            .borrow_mut()
            .push_front((self.children_node, self.current_node, input));
    }

    /// Schedule this child to be visited later (FIFO), i.e., breadth-first.
    /// Useful for exploring siblings before going deeper.
    #[inline]
    pub fn schedule_deferred(&self, input: I) {
        self.stack
            .borrow_mut()
            .push_back((self.children_node, self.current_node, input));
    }
}

impl<'a, E, I> AsRef<E> for WalkerNodeHandle<'a, E, I> {
    fn as_ref(&self) -> &E {
        &self.elem
    }
}

impl<'a, E, I> Deref for WalkerNodeHandle<'a, E, I> {
    type Target = E;

    fn deref(&self) -> &Self::Target {
        &self.elem
    }
}

/// Lightweight handle passed to the visitor, representing the current node.
///
/// - `E` is the underlying expression reference type (here [`AnyExprRef`]).
/// - `I` is the user-defined input/state type threaded through the traversal.
///
/// You can deref or `as_ref()` this handle to access the underlying expression. You
/// can also directly match on this handle to access its children as if you were using
/// [`ExprView`]. Additionally, you can call [`schedule_parent`](Self::schedule_parent_immediate) to
/// enqueue the parent node for a later re-visit.
///
pub struct WalkerHandle<'a, E, I> {
    stack: &'a RefCell<VecDeque<(TreeBufNodeRef, TreeBufNodeRef, I)>>,
    parent: TreeBufNodeRef,
    current_node: TreeBufNodeRef,
    view: ExprView<
        WalkerNodeHandle<'a, E, I>,
        WalkerNodeHandle<'a, E, I>,
        WalkerNodeHandle<'a, E, I>,
    >,
}

impl<'a, E, I> WalkerHandle<'a, E, I> {
    /// Schedule the parent node to be visited immediately. Notice that if this is called within the
    /// iteration of a child, the parent will be revisited immediately after the current node's processing.
    ///
    /// You can make use of the input/state [`I`] to only schedule parent after the visitation of the last
    /// child to achieve a post-order traversal. Callers must ensure that this is not invoked on the root node.
    /// You can check if this node is the root with [`is_root`](Self::is_root).
    #[inline]
    pub fn schedule_parent_immediate(&self, input: I) {
        assert!(
            self.parent != TreeBuf::INVALID_NODE_REF,
            "Cannot schedule parent of root node"
        );

        self.stack
            .borrow_mut()
            .push_front((self.parent, self.current_node, input));
    }

    /// Check if this node is the root of the expression tree.
    #[inline]
    pub fn is_root(&self) -> bool {
        self.parent == TreeBuf::INVALID_NODE_REF
    }
}

impl<'a, E, I> Deref for WalkerHandle<'a, E, I> {
    type Target = ExprView<
        WalkerNodeHandle<'a, E, I>,
        WalkerNodeHandle<'a, E, I>,
        WalkerNodeHandle<'a, E, I>,
    >;

    fn deref(&self) -> &Self::Target {
        &self.view
    }
}

impl<'a, E, I>
    AsRef<
        ExprView<
            WalkerNodeHandle<'a, E, I>,
            WalkerNodeHandle<'a, E, I>,
            WalkerNodeHandle<'a, E, I>,
        >,
    > for WalkerHandle<'a, E, I>
{
    fn as_ref(
        &self,
    ) -> &ExprView<WalkerNodeHandle<'a, E, I>, WalkerNodeHandle<'a, E, I>, WalkerNodeHandle<'a, E, I>>
    {
        &self.view
    }
}

/// Walk an encoded expression in a DFS, user-scheduled manner.
///
/// The visitor receives the current input/state `I` and an [`ExprView`] of `WalkerHandle`s to
/// children. Call [`WalkerHandle::schedule_visit`] on the children you want to traverse.
///
/// Determinism: children scheduled for a node are visited in ascending order of their
/// underlying buffer index; with DFS and a LIFO stack this yields a stable pre-order across runs.
pub fn walk<F, I>(expr: AnyExprRef, input: I, mut walker: F)
where
    F: FnMut(I, WalkerHandle<'_, AnyExprRef<'_>, I>),
{
    // Stack of (node_ref, parent, input)
    let stack = RefCell::new(VecDeque::<(TreeBufNodeRef, TreeBufNodeRef, I)>::new());

    // Add root node to the stack
    stack
        .borrow_mut()
        .push_front((expr.node, TreeBuf::INVALID_NODE_REF, input));

    // Traverse the tree in a depth-first manner
    loop {
        // Pop with a short-lived mutable borrow to avoid overlapping borrows
        let next = {
            let mut s = stack.borrow_mut();
            s.pop_front()
        };
        let Some((current_node, parent, input)) = next else {
            break;
        };

        // Extract the node from the reference
        let node = AnyExpr::_view(expr.tree, current_node).map(
            |elem| WalkerNodeHandle {
                stack: &stack,
                children_node: elem.node,
                current_node,
                elem,
            },
            |elem| WalkerNodeHandle {
                stack: &stack,
                children_node: elem.node,
                current_node,
                elem,
            },
            |elem| WalkerNodeHandle {
                stack: &stack,
                children_node: elem.node,
                current_node,
                elem,
            },
        );
        // Apply the walker function
        walker(
            input,
            WalkerHandle {
                stack: &stack,
                parent,
                current_node,
                view: node,
            },
        );
    }
}

/// Convenience when no input/state needs to be threaded.
#[inline]
pub fn walk_no_input<F>(expr: AnyExprRef, mut walker: F)
where
    F: FnMut(WalkerHandle<'_, AnyExprRef<'_>, ()>),
{
    walk(expr, (), |(), node| walker(node));
}

// pub fn walk_mut<F, I, E>(expr: &mut AnyExpr, input: I, mut walker: F)
// where
//     F: FnMut(
//         I,
//         ExprView<
//             WalkerNodeHandle<'_, AnyExprRef<'_>, I>,
//             WalkerNodeHandle<'_, AnyExprRef<'_>, I>,
//             WalkerNodeHandle<'_, AnyExprRef<'_>, I>,
//         >,
//     ) -> Option<E>,
//     E: EncodableExpr,
// {
//     // TODO: Allow to schedule_parent
//     //       Allow custom iteration order (BFS, etc)

//     let children_iter_buffer: RefCell<
//         StaticVec<(TreeBufNodeRef, I), { TreeBuf::MAX_NUM_REFERENCES }>,
//     > = RefCell::new(Default::default());
//     let mut stack = Vec::<(TreeBufNodeRef, I)>::new();

//     // Add root node to the stack
//     stack.push((expr.tree.root().unwrap(), input));

//     // Traverse the tree in a depth-first manner
//     while let Some((node_ref, input)) = stack.pop() {}
// }

// fn test(expr: &mut AnyExpr) {
//     walk_mut(expr, (), |_, input| match input {
//         ExprView::Bool => Some(True.encode()),
//         ExprView::And(lhs, _) => {
//             lhs.schedule_visit(());
//             None
//         }
//         ExprView::Powerset(_) => None,
//         _ => None,
//     });
// }

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

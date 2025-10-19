use std::{cell::RefCell, mem::swap, ops::Deref};

use crate::{
    encoding::{
        EncodableExpr,
        tree::{TreeBuf, TreeBufNodeRef},
    },
    expr::{AnyExpr, Expr, view::ExprView},
    walker::internal::{InternalWalkerHandle, InternalWalkerNodeHandle, WalkerStackType},
};

pub struct WeakExprRef<'a> {
    node_ref: TreeBufNodeRef,
    tree_ptr: *const TreeBuf,
    tree: &'a RefCell<AnyExpr>,
}

impl EncodableExpr for WeakExprRef<'_> {
    fn encode_tree_step(&self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        assert!(
            std::ptr::addr_eq(self.tree_ptr, tree as *const _),
            "Cannot encode WeakExprRef into a different tree"
        );
        self.node_ref
    }
}

impl Expr for WeakExprRef<'_> {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        // Borrow the view from the underlying tree
        let element = self.tree.borrow();
        AnyExpr::_view(&element.tree, self.node_ref).map_unary(|elem, _| WeakExprRef {
            node_ref: elem.node,
            tree_ptr: self.tree_ptr,
            tree: self.tree,
        })
    }
}

pub struct WalkerNodeHandleMut<'a, I> {
    internal: InternalWalkerNodeHandle<'a, I>,
    elem: WeakExprRef<'a>,
    state: &'a RefCell<State>,
}

impl<'a, I> WalkerNodeHandleMut<'a, I> {
    fn _schedule_any(state: &RefCell<State>) {
        let mut state = state.borrow_mut();
        assert!(
            matches!(*state, State::Unmodified | State::ScheduleChild),
            "Cannot schedule a child after the current node has been modified"
        );
        *state = State::ScheduleChild;
    }

    /// Schedule this child to be visited immediately (LIFO), i.e., depth-first.
    /// Useful for drilling down before exploring siblings.
    #[inline]
    pub fn schedule_immediate(&self, input: I) {
        Self::_schedule_any(self.state);
        self.internal.schedule_immediate(input);
    }

    /// Schedule this child to be visited later (FIFO), i.e., breadth-first.
    /// Useful for exploring siblings before going deeper.
    #[inline]
    pub fn schedule_deferred(&self, input: I) {
        Self::_schedule_any(self.state);
        self.internal.schedule_deferred(input);
    }
}

impl<'a, I> AsRef<WeakExprRef<'a>> for WalkerNodeHandleMut<'a, I> {
    fn as_ref(&self) -> &WeakExprRef<'a> {
        &self.elem
    }
}

impl<'a, I> Deref for WalkerNodeHandleMut<'a, I> {
    type Target = WeakExprRef<'a>;

    fn deref(&self) -> &Self::Target {
        &self.elem
    }
}

#[derive(PartialEq, Eq)]
enum State {
    Unmodified,
    ScheduleChild,
    Modified,
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
pub struct WalkerHandleMut<'a, I> {
    internal: InternalWalkerHandle<'a, I>,
    state: &'a RefCell<State>,
    view: Option<
        ExprView<
            WalkerNodeHandleMut<'a, I>,
            WalkerNodeHandleMut<'a, I>,
            WalkerNodeHandleMut<'a, I>,
        >,
    >,
    tree: &'a RefCell<AnyExpr>,
}

impl<'a, I> WalkerHandleMut<'a, I> {
    fn update_internal(&mut self, expr: impl Expr) -> u16 {
        {
            let mut state = self.state.borrow_mut();
            assert!(
                matches!(*state, State::Unmodified),
                "Cannot update the current node multiple times within the same visitation"
            );

            *state = State::Modified;
        }

        // Add the internal pointer to the tree
        let mut expr_mut = self.tree.borrow_mut();
        let new_node = expr.encode_tree_step(&mut expr_mut.tree);

        // Update the parent of the current node
        if self.internal.parent == TreeBuf::INVALID_NODE_REF {
            // Root node
            expr_mut.tree.set_root(new_node);
        } else {
            // Non-root node: find the parent and update the child reference pointing to the current node.
            let (_, _, references) = expr_mut.tree.get_node(self.internal.parent);

            // Ensure there is only a single reference to the current node
            debug_assert!(
                references
                    .iter()
                    .filter(|r| **r == self.internal.current_node)
                    .count()
                    == 1,
                "Parent node has multiple references to the current node"
            );

            // Update the reference in the parent node
            for i in 0..references.len() {
                if references[i] == self.internal.current_node {
                    expr_mut
                        .tree
                        .update_node_reference(self.internal.parent, i as u8, new_node);
                    break;
                }
            }
        }

        new_node
    }

    /// Schedule the parent node to be visited immediately. Notice that if this is called within the
    /// iteration of a child, the parent will be revisited immediately after the current node's processing.
    ///
    /// You can make use of the input/state [`I`] to only schedule parent after the visitation of the last
    /// child to achieve a post-order traversal. Callers must ensure that this is not invoked on the root node.
    /// You can check if this node is the root with [`is_root`](Self::is_root).
    #[inline]
    pub fn schedule_parent_immediate(&self, input: I) {
        self.internal.schedule_parent_immediate(input);
    }

    #[inline]
    pub fn is_root(&self) -> bool {
        self.internal.is_root()
    }

    pub fn schedule_self_immediate(&self, input: I) {
        WalkerNodeHandleMut::<I>::_schedule_any(self.state);
        self.internal.schedule_self_immediate(input);
    }

    pub fn update_and_reschedule(mut self, expr: impl Expr, input: I) {
        let new_node = self.update_internal(expr);
        self.internal.current_node = new_node;
        self.internal.schedule_self_immediate(input);
    }

    pub fn update(mut self, expr: impl Expr) {
        self.update_internal(expr);
    }
}

impl<'a, I>
    AsRef<
        ExprView<
            WalkerNodeHandleMut<'a, I>,
            WalkerNodeHandleMut<'a, I>,
            WalkerNodeHandleMut<'a, I>,
        >,
    > for WalkerHandleMut<'a, I>
{
    #[inline]
    fn as_ref(
        &self,
    ) -> &ExprView<WalkerNodeHandleMut<'a, I>, WalkerNodeHandleMut<'a, I>, WalkerNodeHandleMut<'a, I>>
    {
        self.view
            .as_ref()
            .expect("Cannot iterate over children after self has been updated")
    }
}

impl<'a, I> Deref for WalkerHandleMut<'a, I> {
    type Target = ExprView<
        WalkerNodeHandleMut<'a, I>,
        WalkerNodeHandleMut<'a, I>,
        WalkerNodeHandleMut<'a, I>,
    >;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

pub fn walk_mut<F, I>(expr: &mut AnyExpr, input: I, mut walker: F)
where
    F: FnMut(I, WalkerHandleMut<'_, I>),
{
    // Stack of (node_ref, parent, input)
    let stack = RefCell::new(WalkerStackType::<I>::new());

    // Add root node to the stack
    stack
        .borrow_mut()
        .push_front((expr.tree.root().unwrap(), TreeBuf::INVALID_NODE_REF, input));

    let mut dummy = AnyExpr {
        tree: TreeBuf::default(),
    };
    swap(expr, &mut dummy);
    let expr_ref_cell = RefCell::new(dummy);
    let modified_state = RefCell::new(State::Unmodified);

    // Traverse the tree
    loop {
        // Pop with a short-lived mutable borrow to avoid overlapping borrows
        let next = {
            let mut s = stack.borrow_mut();
            s.pop_front()
        };
        let Some((current_node, parent, input)) = next else {
            break;
        };

        let tree = &expr_ref_cell.borrow().tree;

        // Extract the node from the reference
        let view = AnyExpr::_view(tree, current_node).map_unary(|elem, _| WalkerNodeHandleMut {
            internal: InternalWalkerNodeHandle {
                stack: &stack,
                children_node: elem.node,
                current_node,
            },
            elem: WeakExprRef {
                node_ref: elem.node,
                tree_ptr: tree as *const _,
                tree: &expr_ref_cell,
            },
            state: &modified_state,
        });

        // Apply the walker function
        walker(
            input,
            WalkerHandleMut {
                internal: InternalWalkerHandle {
                    stack: &stack,
                    parent,
                    current_node,
                },
                state: &modified_state,
                view: Some(view),
                tree: &expr_ref_cell,
            },
        );

        // If the current node was modified
        if modified_state.replace(State::Unmodified) == State::Modified {
            // TODO: Figure out what to do with stale references on the stack
            // TODO: Currently, the way the WeakExprRef is designed, we cannot have multiple references to the same child.
            // TODO: What to do about stale reference on the stack ?
        }
    }

    // Restore the modified expression
    let mut final_expr = expr_ref_cell.into_inner();
    swap(expr, &mut final_expr);
}

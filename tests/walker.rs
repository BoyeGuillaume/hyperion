use hyformal::expr::defs::*;
use hyformal::expr::func;
use hyformal::expr::variant::ExprType;
use hyformal::expr::*;
use hyformal::variable::InlineVariable;
use hyformal::walker::{WalkerHandle, walk_no_input};

fn schedule_all_with<I: Clone>(
    node: WalkerHandle<'_, hyformal::expr::AnyExprRef<'_>, I>,
    input: I,
) {
    use view::ExprView::*;
    match node.as_ref() {
        Not(c) | Powerset(c) => c.schedule_immediate(input),
        And(l, r)
        | Or(l, r)
        | Implies(l, r)
        | Iff(l, r)
        | Equal(l, r)
        | Tuple(l, r)
        | Lambda { arg: l, body: r }
        | Call { func: l, arg: r }
        | Forall {
            dtype: l, inner: r, ..
        }
        | Exists {
            dtype: l, inner: r, ..
        } => {
            l.schedule_immediate(input.clone());
            r.schedule_immediate(input);
        }
        If {
            condition,
            then_branch,
            else_branch,
        } => {
            condition.schedule_immediate(input.clone());
            then_branch.schedule_immediate(input.clone());
            else_branch.schedule_immediate(input);
        }
        _ => {}
    }
}

#[test]
fn walk_counts_nodes_small() {
    // And(True, Not(False)) => 4 nodes
    let expr = (True & !False).encode();
    let mut count = 0usize;
    walk_no_input(expr.as_ref(), |node| {
        schedule_all_with::<()>(node, ());
        count += 1;
    });
    assert_eq!(count, 4);
}

#[test]
fn walk_respects_ordering_preorder() {
    // Structure: If(cond=True, then=(Bool, True), else=Not(False))
    let encoded = If {
        condition: True,
        then_branch: hyformal::expr::func::tuple(Bool, True),
        else_branch: Not { inner: False },
    }
    .encode();

    // Record node types and compare multiset (ordering is implementation-defined by buffer indices)
    let mut types = Vec::new();
    walk_no_input(encoded.as_ref(), |node| {
        types.push(node.type_());
        schedule_all_with::<()>(node, ());
    });
    assert!(!types.is_empty());
    assert_eq!(types[0], ExprType::If);
    // Build histogram
    let mut counts = std::collections::HashMap::<ExprType, usize>::new();
    for t in types {
        *counts.entry(t).or_insert(0) += 1;
    }
    // Expect exactly one If, one Tuple, one Bool, two True (one as condition, one in tuple), one Not, one False
    assert_eq!(*counts.get(&ExprType::If).unwrap_or(&0), 1);
    assert_eq!(*counts.get(&ExprType::Tuple).unwrap_or(&0), 1);
    assert_eq!(*counts.get(&ExprType::Bool).unwrap_or(&0), 1);
    assert_eq!(*counts.get(&ExprType::True).unwrap_or(&0), 2);
    assert_eq!(*counts.get(&ExprType::Not).unwrap_or(&0), 1);
    assert_eq!(*counts.get(&ExprType::False).unwrap_or(&0), 1);
}

#[test]
fn walk_with_state_prunes() {
    // Deep-ish binary tree: repeat And nesting to depth d
    fn deep_and(mut d: usize) -> AnyExpr {
        let mut e: AnyExpr = True.encode();
        while d > 0 {
            e = hyformal::expr::func::and(e.as_ref(), True).encode();
            d -= 1;
        }
        e
    }

    let expr = deep_and(32);
    // Limit global budget to 5 nodes; ensure we stop exploring further
    let mut remaining = 5u32;
    let mut visited = 0u32;
    walk_no_input(expr.as_ref(), |node| {
        if remaining == 0 {
            return;
        }
        visited += 1;
        remaining -= 1;
        if remaining == 0 {
            return;
        }
        // Explore all children while we still have global budget
        schedule_all_with::<()>(node, ());
    });
    assert_eq!(visited, 5);
}

#[test]
fn walk_large_tree_smoke() {
    // Build a reasonably large expression (~thousands of nodes)
    // Shape: fold over a sequence alternating constructors to create width and depth
    let mut e: AnyExpr = True.encode();
    for i in 0..2000u32 {
        // produces ~O(n) nodes
        println!(
            "Building step {}/2000: {} - estimated wasted bytes: {}",
            i,
            e.storage_size(),
            e.estimated_wasted_bytes()
        );
        let var = InlineVariable::new_from_raw(i % 10);
        let dtype = Bool; // keep a consistent static type for generics
        let quant = hyformal::expr::func::forall(var, dtype, e.as_ref());
        let t = True; // keep a consistent constructor type
        e = (quant.equals(t)).encode();
    }

    // Traverse all nodes and count; this should complete quickly with no allocations surprises
    let mut count = 0usize;
    walk_no_input(e.as_ref(), |node| {
        schedule_all_with::<()>(node, ());
        count += 1;
    });

    // Expect a sizable count; exact number depends on construction but must be > baseline
    assert!(count > 5000, "expected many nodes visited, got {count}");
}

#[test]
fn walk_matches_encode_view_types() {
    // Sanity: ensure that node.type_ matches view().type_ across traversal
    let x = InlineVariable::new_from_raw(1);
    let expr = hyformal::expr::func::forall(x, Bool, True.equals(False)).encode();
    let mut ok = true;
    walk_no_input(expr.as_ref(), |node| {
        let t = node.type_();
        // Reconstruct via viewing the underlying AnyExprRef
        match node.as_ref() {
            &view::ExprView::Bool => assert_eq!(t, ExprType::Bool),
            &view::ExprView::Omega => assert_eq!(t, ExprType::Omega),
            &view::ExprView::True => assert_eq!(t, ExprType::True),
            &view::ExprView::False => assert_eq!(t, ExprType::False),
            &view::ExprView::Never => assert_eq!(t, ExprType::Never),
            &view::ExprView::Not(_) => assert_eq!(t, ExprType::Not),
            &view::ExprView::Powerset(_) => assert_eq!(t, ExprType::Powerset),
            &view::ExprView::And(_, _) => assert_eq!(t, ExprType::And),
            &view::ExprView::Or(_, _) => assert_eq!(t, ExprType::Or),
            &view::ExprView::Implies(_, _) => assert_eq!(t, ExprType::Implies),
            &view::ExprView::Iff(_, _) => assert_eq!(t, ExprType::Iff),
            &view::ExprView::Equal(_, _) => assert_eq!(t, ExprType::Equal),
            &view::ExprView::Lambda { .. } => assert_eq!(t, ExprType::Lambda),
            &view::ExprView::Call { .. } => assert_eq!(t, ExprType::Call),
            &view::ExprView::Tuple(_, _) => assert_eq!(t, ExprType::Tuple),
            &view::ExprView::Forall { .. } => assert_eq!(t, ExprType::Forall),
            &view::ExprView::Exists { .. } => assert_eq!(t, ExprType::Exists),
            &view::ExprView::If { .. } => assert_eq!(t, ExprType::If),
            &view::ExprView::Variable(_) => assert_eq!(t, ExprType::Variable),
        }
        ok &= true;
    });
    assert!(ok);
}

fn build_complex_expr() -> AnyExpr {
    // Root: If
    // - condition: And(Bool, True)
    // - then: Tuple(
    //       Lambda(arg=Bool, body=Call(func=Var(v1), arg=Tuple(True, Bool))),
    //       Powerset(False)
    //   )
    // - else: Forall(v2, Bool, Exists(v3, Omega, Not(Var(v4))))
    let v1 = InlineVariable::new_from_raw(1);
    let v2 = InlineVariable::new_from_raw(2);
    let v3 = InlineVariable::new_from_raw(3);
    let v4 = InlineVariable::new_from_raw(4);

    let condition = func::and(Bool, True);

    let call_arg = func::tuple(True, Bool);
    let call = func::call(v1, call_arg);
    let lambda = func::lambda(Bool, call);
    let then_branch = func::tuple(lambda, func::powerset(False));

    let exists_inner = Not { inner: v4 };
    let exists = func::exists(v3, Omega, exists_inner);
    let else_branch = func::forall(v2, Bool, exists);

    let root = func::branch(condition, then_branch, else_branch);
    root.encode()
}

fn schedule_children_lr_bfs(node: WalkerHandle<'_, AnyExprRef<'_>, ()>) {
    use view::ExprView::*;
    match node.as_ref() {
        Not(c) | Powerset(c) => c.schedule_deferred(()),
        And(l, r)
        | Or(l, r)
        | Implies(l, r)
        | Iff(l, r)
        | Equal(l, r)
        | Tuple(l, r)
        | Lambda { arg: l, body: r }
        | Call { func: l, arg: r }
        | Forall {
            dtype: l, inner: r, ..
        }
        | Exists {
            dtype: l, inner: r, ..
        } => {
            l.schedule_deferred(());
            r.schedule_deferred(());
        }
        If {
            condition,
            then_branch,
            else_branch,
        } => {
            condition.schedule_deferred(());
            then_branch.schedule_deferred(());
            else_branch.schedule_deferred(());
        }
        _ => {}
    }
}

fn schedule_children_lr_dfs(node: WalkerHandle<'_, AnyExprRef<'_>, ()>) {
    use view::ExprView::*;
    match node.as_ref() {
        Not(c) | Powerset(c) => c.schedule_immediate(()),
        And(l, r)
        | Or(l, r)
        | Implies(l, r)
        | Iff(l, r)
        | Equal(l, r)
        | Tuple(l, r)
        | Lambda { arg: l, body: r }
        | Call { func: l, arg: r }
        | Forall {
            dtype: l, inner: r, ..
        }
        | Exists {
            dtype: l, inner: r, ..
        } => {
            // Reverse order for immediate (LIFO) to visit left-to-right
            r.schedule_immediate(());
            l.schedule_immediate(());
        }
        If {
            condition,
            then_branch,
            else_branch,
        } => {
            // Reverse scheduling: else, then, condition -> visit order: condition, then, else
            else_branch.schedule_immediate(());
            then_branch.schedule_immediate(());
            condition.schedule_immediate(());
        }
        _ => {}
    }
}

#[test]
fn walk_order_complex_bfs() {
    let e = build_complex_expr();
    let mut seen = Vec::<ExprType>::new();
    walk_no_input(e.as_ref(), |node| {
        seen.push(node.type_());
        schedule_children_lr_bfs(node);
    });

    // Expected BFS left-to-right by levels
    let expected = vec![
        ExprType::If,
        // Level 1
        ExprType::And,
        ExprType::Tuple,
        ExprType::Forall,
        // Level 2
        ExprType::Bool,
        ExprType::True,
        ExprType::Lambda,
        ExprType::Powerset,
        ExprType::Bool,
        ExprType::Exists,
        // Level 3
        ExprType::Bool,
        ExprType::Call,
        ExprType::False,
        ExprType::Omega,
        ExprType::Not,
        // Level 4
        ExprType::Variable,
        ExprType::Tuple,
        ExprType::Variable,
        // Level 5
        ExprType::True,
        ExprType::Bool,
    ];

    assert_eq!(seen, expected,);
}

#[test]
fn walk_order_complex_dfs_preorder() {
    let e = build_complex_expr();
    let mut seen = Vec::<ExprType>::new();
    walk_no_input(e.as_ref(), |node| {
        seen.push(node.type_());
        schedule_children_lr_dfs(node);
    });

    // Expected DFS pre-order, left-to-right
    let expected = vec![
        ExprType::If,
        // cond subtree
        ExprType::And,
        ExprType::Bool,
        ExprType::True,
        // then subtree
        ExprType::Tuple,
        ExprType::Lambda,
        ExprType::Bool,
        ExprType::Call,
        ExprType::Variable,
        ExprType::Tuple,
        ExprType::True,
        ExprType::Bool,
        ExprType::Powerset,
        ExprType::False,
        // else subtree
        ExprType::Forall,
        ExprType::Bool,
        ExprType::Exists,
        ExprType::Omega,
        ExprType::Not,
        ExprType::Variable,
    ];

    assert_eq!(seen, expected,);
}

#[test]
fn walk_schedule_parent_post_order_without_loops() {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum State {
        Visit(bool),
        Post,
    }

    let e = build_complex_expr();
    let mut post = Vec::<ExprType>::new();

    hyformal::walker::walk(e.as_ref(), State::Visit(false), |state, node| {
        use view::ExprView::*;
        match state {
            State::Post => {
                // Post-visit: record and don't schedule more
                post.push(node.type_());
            }
            State::Visit(is_last) => {
                println!("Visiting node: {:?}, is_last: {}", node.type_(), is_last);
                // If this node is marked as the last child, schedule its parent for a post-visit now.
                if is_last && !node.is_root() {
                    node.schedule_parent_immediate(State::Post);
                }
                match node.as_ref() {
                    Not(c) | Powerset(c) => c.schedule_immediate(State::Visit(true)),
                    And(l, r)
                    | Or(l, r)
                    | Implies(l, r)
                    | Iff(l, r)
                    | Equal(l, r)
                    | Tuple(l, r)
                    | Lambda { arg: l, body: r }
                    | Call { func: l, arg: r }
                    | Forall {
                        dtype: l, inner: r, ..
                    }
                    | Exists {
                        dtype: l, inner: r, ..
                    } => {
                        // reverse scheduling for DFS; mark right as last
                        r.schedule_immediate(State::Visit(true));
                        l.schedule_immediate(State::Visit(false));
                    }
                    If {
                        condition,
                        then_branch,
                        else_branch,
                    } => {
                        // reverse scheduling; mark else as last
                        else_branch.schedule_immediate(State::Visit(true));
                        then_branch.schedule_immediate(State::Visit(false));
                        condition.schedule_immediate(State::Visit(false));
                    }
                    _ => {}
                }
            }
        }
    });

    let expected_post = vec![
        // If.cond > And (scheduled by True)
        ExprType::And,
        // If.then > Tuple.0 > Lambda.body > Call.arg > Tuple (scheduled by Bool)
        ExprType::Tuple,
        // If.then > Tuple.0 > Lambda.body > Call (scheduled by Tuple)
        ExprType::Call,
        // If.then > Tuple.0 > Lambda (scheduled by Call)
        ExprType::Lambda,
        // If.then > Tuple.1 > Powerset (scheduled by False)
        ExprType::Powerset,
        // If.then > Tuple (scheduled by Powerset)
        ExprType::Tuple,
        // If.else > Forall.inner > Exists.inner > Not (scheduled by Var)
        ExprType::Not,
        // If.else > Forall.inner > Exists (scheduled by Not)
        ExprType::Exists,
        // If.else > Forall (scheduled by Exists)
        ExprType::Forall,
        // If (scheduled by Forall)
        ExprType::If,
    ];

    assert_eq!(post, expected_post,);
}
#[test]
fn walk_order_immediate_is_dfs_on_siblings() {
    // Root with 3 children to make sibling-order observable
    let encoded = If {
        condition: True,
        then_branch: hyformal::expr::func::tuple(Bool, True),
        else_branch: Not { inner: False },
    }
    .encode();

    let mut seen = Vec::<ExprType>::new();
    walk_no_input(encoded.as_ref(), |node| {
        // Record visit order
        seen.push(node.type_());
        // Only schedule root's children to isolate sibling order
        if let view::ExprView::If {
            condition,
            then_branch,
            else_branch,
        } = node.as_ref()
        {
            // Schedule all as immediate in the natural (cond, then, else) order.
            // Because of LIFO, visit order of siblings should be: else, then, cond.
            condition.schedule_immediate(());
            then_branch.schedule_immediate(());
            else_branch.schedule_immediate(());
        }
    });

    // Expect traversal to start at If, then siblings in reverse scheduling order due to LIFO.
    assert!(seen.len() >= 4);
    assert_eq!(seen[0], ExprType::If);
    assert_eq!(seen[1], ExprType::Not); // else first
    assert_eq!(seen[2], ExprType::Tuple); // then second
    assert_eq!(seen[3], ExprType::True); // condition last
}

#[test]
fn walk_order_deferred_is_bfs_on_siblings() {
    let encoded = If {
        condition: True,
        then_branch: hyformal::expr::func::tuple(Bool, True),
        else_branch: Not { inner: False },
    }
    .encode();

    let mut seen = Vec::<ExprType>::new();
    walk_no_input(encoded.as_ref(), |node| {
        seen.push(node.type_());
        if let view::ExprView::If {
            condition,
            then_branch,
            else_branch,
        } = node.as_ref()
        {
            // Schedule all as deferred in the natural (cond, then, else) order.
            // Because of FIFO, visit order should be: cond, then, else.
            condition.schedule_deferred(());
            then_branch.schedule_deferred(());
            else_branch.schedule_deferred(());
        }
    });

    assert!(seen.len() >= 4);
    assert_eq!(seen[0], ExprType::If);
    assert_eq!(seen[1], ExprType::True); // condition first
    assert_eq!(seen[2], ExprType::Tuple); // then second
    assert_eq!(seen[3], ExprType::Not); // else third
}

#[test]
fn walk_order_mixed_immediate_and_deferred() {
    let encoded = If {
        condition: True,
        then_branch: hyformal::expr::func::tuple(Bool, True),
        else_branch: Not { inner: False },
    }
    .encode();

    let mut seen = Vec::<ExprType>::new();
    walk_no_input(encoded.as_ref(), |node| {
        seen.push(node.type_());
        if let view::ExprView::If {
            condition,
            then_branch,
            else_branch,
        } = node.as_ref()
        {
            // Mix: immediate on cond and else, deferred on then.
            // Push order: cond(immediate), then(deferred), else(immediate)
            // Stack front after root: else (last immediate), then cond, and back contains 'then'.
            // So order: If, else, cond, then.
            condition.schedule_immediate(());
            then_branch.schedule_deferred(());
            else_branch.schedule_immediate(());
        }
    });

    assert!(seen.len() >= 4);
    assert_eq!(seen[0], ExprType::If);
    assert_eq!(seen[1], ExprType::Not); // else (immediate, last pushed)
    assert_eq!(seen[2], ExprType::True); // condition (immediate, first pushed)
    assert_eq!(seen[3], ExprType::Tuple); // then (deferred)
}

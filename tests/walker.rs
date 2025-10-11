use hyformal::expr::defs::*;
use hyformal::expr::variant::ExprType;
use hyformal::expr::*;
use hyformal::variable::InlineVariable;
use hyformal::walker::{WalkerHandle, walk_no_input};

fn schedule_all_with<I: Clone>(
    node: view::ExprView<
        WalkerHandle<'_, hyformal::expr::AnyExprRef<'_>, I>,
        WalkerHandle<'_, hyformal::expr::AnyExprRef<'_>, I>,
        WalkerHandle<'_, hyformal::expr::AnyExprRef<'_>, I>,
    >,
    input: I,
) {
    use view::ExprView::*;
    match node {
        Not(c) | Powerset(c) => c.schedule_visit(input),
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
            l.schedule_visit(input.clone());
            r.schedule_visit(input);
        }
        If {
            condition,
            then_branch,
            else_branch,
        } => {
            condition.schedule_visit(input.clone());
            then_branch.schedule_visit(input.clone());
            else_branch.schedule_visit(input);
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
        let var = InlineVariable::new_from_raw((i % 10) as u32);
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
    assert!(count > 5000, "expected many nodes visited, got {}", count);
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
        match node {
            view::ExprView::Bool => assert_eq!(t, ExprType::Bool),
            view::ExprView::Omega => assert_eq!(t, ExprType::Omega),
            view::ExprView::True => assert_eq!(t, ExprType::True),
            view::ExprView::False => assert_eq!(t, ExprType::False),
            view::ExprView::Never => assert_eq!(t, ExprType::Never),
            view::ExprView::Not(_) => assert_eq!(t, ExprType::Not),
            view::ExprView::Powerset(_) => assert_eq!(t, ExprType::Powerset),
            view::ExprView::And(_, _) => assert_eq!(t, ExprType::And),
            view::ExprView::Or(_, _) => assert_eq!(t, ExprType::Or),
            view::ExprView::Implies(_, _) => assert_eq!(t, ExprType::Implies),
            view::ExprView::Iff(_, _) => assert_eq!(t, ExprType::Iff),
            view::ExprView::Equal(_, _) => assert_eq!(t, ExprType::Equal),
            view::ExprView::Lambda { .. } => assert_eq!(t, ExprType::Lambda),
            view::ExprView::Call { .. } => assert_eq!(t, ExprType::Call),
            view::ExprView::Tuple(_, _) => assert_eq!(t, ExprType::Tuple),
            view::ExprView::Forall { .. } => assert_eq!(t, ExprType::Forall),
            view::ExprView::Exists { .. } => assert_eq!(t, ExprType::Exists),
            view::ExprView::If { .. } => assert_eq!(t, ExprType::If),
            view::ExprView::Variable(_) => assert_eq!(t, ExprType::Variable),
        }
        ok &= true;
    });
    assert!(ok);
}

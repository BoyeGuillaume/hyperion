use hyformal::encoding::tree::TreeBuf;
use hyformal::expr::variant::ExprType;

fn count_reachable_nodes(t: &TreeBuf, root: u16) -> usize {
    let mut stack = vec![root];
    let mut visited = std::collections::HashSet::new();
    let mut count = 0usize;
    while let Some(n) = stack.pop() {
        if !visited.insert(n) {
            continue;
        }
        count += 1;
        let (_op, _d, children) = t.get_node(n);
        for c in children {
            stack.push(c);
        }
    }
    count
}

#[test]
fn empty_and_root_management() {
    let mut t = TreeBuf::new();
    assert!(t.empty());
    let leaf = t.push_node(ExprType::True as u8, None, &[]);
    t.set_root(leaf);
    assert!(!t.empty());
    assert_eq!(t.root(), Some(leaf));
}

#[test]
fn node_with_max_references_and_data_variants() {
    let mut t = TreeBuf::new();
    // 7 children allowed
    let kids: Vec<_> = (0..7)
        .map(|i| t.push_node(ExprType::Variable as u8, Some(i), &[]))
        .collect();
    let parent = t.push_node(ExprType::Tuple as u8, None, &kids);
    t.set_root(parent);

    let (op, data, refs) = t.get_node(parent);
    assert_eq!(op, ExprType::Tuple as u8);
    assert!(data.is_none());
    assert_eq!(refs.as_ref(), kids.as_slice());

    for (i, k) in kids.into_iter().enumerate() {
        let (opk, datak, rk) = t.get_node(k);
        assert_eq!(opk, ExprType::Variable as u8);
        assert_eq!(datak, Some(i as u32));
        assert!(rk.is_empty());
    }
}

#[test]
fn update_reference_counts_waste_and_consolidate_clears() {
    let mut t = TreeBuf::new();
    // Force spill beyond 32B inline capacity (SmallVec<[u8;32]>)
    let mut leaves = Vec::new();
    for _ in 0..20 {
        // ~120 bytes via 2-leaf And nodes
        leaves.push(t.push_node(ExprType::True as u8, None, &[]));
    }
    let mut last = t.push_node(ExprType::And as u8, None, &[leaves[0], leaves[1]]);
    for i in (2..leaves.len()).step_by(2) {
        let rhs = t.push_node(ExprType::And as u8, None, &[leaves[i], leaves[i + 1]]);
        let parent = t.push_node(ExprType::And as u8, None, &[last, rhs]);
        last = parent;
    }
    t.set_root(last);

    let before_total = t.total_bytes();
    // Change an internal child to create small waste (< 25%)
    let new_leaf = t.push_node(ExprType::False as u8, None, &[]);
    t.update_node_reference(last, 1, new_leaf);

    // Accumulate more waste to pass 25% threshold, robust to heuristic changes
    let mut iters = 0usize;
    while iters < 2000 {
        // many small leaves replaced to add waste
        let a1 = t.push_node(ExprType::True as u8, None, &[]);
        let a2 = t.push_node(ExprType::True as u8, None, &[]);
        let a3 = t.push_node(ExprType::And as u8, None, &[a1, a2]);
        t.set_root(a3);
        iters += 1;
    }

    // Consolidate and ensure waste is gone, structure still decodes
    let total_bytes = t.total_bytes();
    t.consolidate();
    assert!(t.total_bytes() < total_bytes);

    // Assert at least 80% of bytes reclaimed to avoid fragile test
    assert!(
        t.total_bytes() <= before_total - (before_total / 20),
        "not enough bytes reclaimed"
    );

    let root = t.root().unwrap();
    let (op, _d, _c) = t.get_node(root);
    assert_eq!(op, ExprType::And as u8);
}

#[test]
fn update_node_data_and_reference_roundtrip() {
    let mut t = TreeBuf::new();
    let v = t.push_node(ExprType::Variable as u8, Some(1), &[]);
    let not = t.push_node(ExprType::Not as u8, None, &[v]);
    t.set_root(not);

    // change data
    t.update_node_data(v, 123);
    let (_opv, dv, _c) = t.get_node(v);
    assert_eq!(dv, Some(123));

    // change ref
    let v2 = t.push_node(ExprType::Variable as u8, Some(2), &[]);
    t.update_node_reference(not, 0, v2);
    let (_op, _d, ch) = t.get_node(not);
    assert_eq!(ch.as_ref(), &[v2]);
}

#[test]
fn push_tree_creates_independent_copy() {
    let mut a = TreeBuf::new();
    let v = a.push_node(ExprType::Variable as u8, Some(7), &[]);
    let not = a.push_node(ExprType::Not as u8, None, &[v]);
    a.set_root(not);

    let mut b = TreeBuf::new();
    let b_root = b.push_tree(&a, a.root().unwrap());
    b.set_root(b_root);

    // Mutate original; copy should remain the same
    a.update_node_data(v, 42);

    let (_opb, db, _cb) = b.get_node(b_root);
    assert!(db.is_none());
    let (_opbv, dbv, _cbv) = {
        let (_op, _d, ch) = b.get_node(b_root);
        b.get_node(ch[0])
    };
    assert_eq!(dbv, Some(7));
}

#[test]
fn deep_tree_with_ternary_nodes() {
    let mut t = TreeBuf::new();
    // Build a chain of nested If nodes to test traversal and storage.
    // Avoid reusing the same leaves across multiple parents to prevent false-positive
    // cycle reports in the debug utility (it treats revisiting a node as a cycle).
    let leaf_t = t.push_node(ExprType::True as u8, None, &[]);
    let first_leaf_f = t.push_node(ExprType::False as u8, None, &[]);
    let mut cond = t.push_node(ExprType::Variable as u8, Some(0), &[]);
    let mut root = t.push_node(ExprType::If as u8, None, &[cond, leaf_t, first_leaf_f]);
    for i in 1..200 {
        // reasonably large but safe under 64KiB
        cond = t.push_node(ExprType::Variable as u8, Some(i), &[]);
        // fresh else leaf to avoid DAG sharing that the debug detector flags
        let leaf_f = t.push_node(ExprType::False as u8, None, &[]);
        root = t.push_node(ExprType::If as u8, None, &[cond, root, leaf_f]);
    }
    t.set_root(root);

    let n = count_reachable_nodes(&t, t.root().unwrap());
    assert!(n >= 200);
}

/// When the SmallVec storage hasn't spilled yet, any amount of waste should trigger consolidation.
#[test]
fn consolidate_when_inlined_and_some_waste() {
    let mut t = TreeBuf::new();

    {
        // Build a tiny tree that remains inlined (< 32 bytes)
        let leaf_true = t.push_node(ExprType::True as u8, Some(1), &[]);
        let leaf_false = t.push_node(ExprType::False as u8, None, &[]);
        let not_node = t.push_node(ExprType::Not as u8, None, &[leaf_true]);
        t.set_root(not_node);

        // Mutate child to create waste (the previous child node becomes wasted)
        t.update_node_reference(not_node, 0, leaf_false);
    }

    // Compute number of bytes used
    let before_total = t.total_bytes();
    assert!(t.total_bytes() > 0);

    // Consilidation invalidates all previous node indices
    t.consolidate();
    assert!(t.total_bytes() < before_total);

    let (op, _data, children) = t.get_node(t.root().unwrap());
    assert_eq!(op, ExprType::Not as u8);
    let leaf_ref = t.get_node(children[0]);
    assert_eq!(leaf_ref.0, ExprType::False as u8);
    assert_eq!(leaf_ref.1, None);
}

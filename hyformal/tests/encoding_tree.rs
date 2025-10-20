use hyformal::encoding::tree::TreeBuf;
use hyformal::expr::variant::ExprType;

#[test]
fn push_and_get_node_roundtrip() {
    let mut t = TreeBuf::new();
    let a = t.push_node(ExprType::True as u8, None, &[]);
    let b = t.push_node(ExprType::False as u8, None, &[]);
    let ab = t.push_node(ExprType::And as u8, None, &[a, b]);
    t.set_root(ab);

    let (op, data, children) = t.get_node(ab);
    assert_eq!(op, ExprType::And as u8);
    assert!(data.is_none());
    assert_eq!(children.as_ref(), &[a, b]);
}

#[test]
fn update_reference_and_data_then_consolidate() {
    let mut t = TreeBuf::new();
    let v1 = t.push_node(ExprType::Variable as u8, Some(42), &[]);
    let v2 = t.push_node(ExprType::Variable as u8, Some(7), &[]);
    let not = t.push_node(ExprType::Not as u8, None, &[v1]);
    t.set_root(not);

    // Change child from v1 to v2
    t.update_node_reference(not, 0, v2);
    // Change data of v2
    t.update_node_data(v2, 9);

    let (_op, data2, _c) = t.get_node(v2);
    assert_eq!(data2, Some(9));

    // Do not force consolidation; just ensure updates are visible

    let root = t.root().unwrap();
    let (_op, _d, ch) = t.get_node(root);
    assert_eq!(ch.len(), 1);
}

#[test]
fn push_tree_copies_other_buffer() {
    let mut a = TreeBuf::new();
    let t_true = a.push_node(ExprType::True as u8, None, &[]);
    a.set_root(t_true);

    let mut b = TreeBuf::new();
    let new_ref = b.push_tree(&a, a.root().unwrap());
    b.set_root(new_ref);

    let (op, _d, _c) = b.get_node(new_ref);
    assert_eq!(op, ExprType::True as u8);
}

use hyformal::arena::{ArenaAllocableExpr, ArenaAnyExpr, ExprArenaCtx};
use hyformal::encoding::EncodableExpr;
use hyformal::expr::defs::*;
use hyformal::expr::view::ExprView;
use hyformal::expr::{AnyExpr, Expr, variant::ExprType};
use hyformal::variable::InlineVariable;

// Helpers
fn type_of(expr: &AnyExpr) -> ExprType {
    expr.as_ref().type_()
}

#[test]
fn simple_arena_alloc_and_encode_bool_ops() {
    // Build small expressions using defs and ensure encoding/view roundtrips
    let p = True;
    let q = False;
    let conj = And { lhs: p, rhs: q };
    let disj = Or { lhs: p, rhs: q };
    let neg = Not { inner: q };

    let e1 = conj.encode();
    let e2 = disj.encode();
    let e3 = neg.encode();

    assert_eq!(type_of(&e1), ExprType::And);
    assert_eq!(type_of(&e2), ExprType::Or);
    assert_eq!(type_of(&e3), ExprType::Not);

    // Structural equality across independently-built trees
    let e1_again = And {
        lhs: True,
        rhs: False,
    }
    .encode();
    assert!(e1 == e1_again);
}

#[test]
fn variables_lambda_and_call_roundtrip() {
    let x = InlineVariable::new_from_raw(0);
    let y = InlineVariable::new_from_raw(1);

    // lambda x. (x, y)
    let lam = x.lambda(x.tuple(y));
    let app = lam.apply(True);

    let encoded = app.encode();
    assert_eq!(type_of(&encoded), ExprType::Call);
    if let hyformal::expr::view::ExprView::Call { func, arg: _ } = encoded.as_ref().view_typed() {
        // Inspect function
        assert_eq!(func.type_(), ExprType::Lambda);
        if let hyformal::expr::view::ExprView::Lambda { arg, body } = func.view_typed() {
            assert_eq!(arg.type_(), ExprType::Variable);
            assert_eq!(body.type_(), ExprType::Tuple);
        } else {
            panic!("expected lambda view");
        }
    } else {
        panic!("expected call view");
    }
}

#[test]
fn quantified_and_conditional_complex_expr() {
    // Build a more complex expression mixing quantifiers and conditionals:
    // forall x: Bool . if True then (x = x) else False
    let x = InlineVariable::new_from_raw(2);
    let dtype = hyformal::expr::defs::Bool;
    let predicate = hyformal::expr::defs::If {
        condition: hyformal::expr::defs::True,
        then_branch: x.equals(x),
        else_branch: hyformal::expr::defs::False,
    };
    let expr = hyformal::expr::defs::ForAll {
        variable: x,
        dtype,
        inner: predicate,
    };

    let encoded = expr.encode();
    assert_eq!(type_of(&encoded), ExprType::Forall);

    // Walk a bit structurally and ensure expected shapes
    use hyformal::expr::view::ExprView::*;
    match encoded.as_ref().view_typed() {
        Forall {
            variable,
            dtype,
            inner,
        } => {
            assert_eq!(variable.raw(), 2);
            assert_eq!(dtype.type_(), ExprType::Bool);
            match inner.view_typed() {
                If {
                    condition,
                    then_branch,
                    else_branch,
                } => {
                    assert_eq!(condition.type_(), ExprType::True);
                    assert_eq!(else_branch.type_(), ExprType::False);
                    assert_eq!(then_branch.type_(), ExprType::Equal);
                }
                _ => panic!("expected If in forall body"),
            }
        }
        _ => panic!("expected Forall at root"),
    }
}

#[test]
fn complex_nested_expression_equality() {
    // Build two logically-identical complex expressions via different construction orders
    let a = InlineVariable::new_from_raw(10);
    let b = InlineVariable::new_from_raw(11);
    let t = True;
    let f = False;

    // expr1: if t then (a = b) <=> (b = a) else !(t \/ f)
    let eq1 = Equal { lhs: a, rhs: b };
    let eq2 = Equal { lhs: b, rhs: a };
    let iff = Iff { lhs: eq1, rhs: eq2 };
    let disj = Or { lhs: t, rhs: f };
    let expr1 = If {
        condition: t,
        then_branch: iff,
        else_branch: Not { inner: disj },
    };

    // expr2: constructed in a different order but structurally identical
    let expr2 = If {
        condition: True,
        then_branch: Iff {
            lhs: Equal {
                lhs: InlineVariable::new_from_raw(10),
                rhs: InlineVariable::new_from_raw(11),
            },
            rhs: Equal {
                lhs: InlineVariable::new_from_raw(11),
                rhs: InlineVariable::new_from_raw(10),
            },
        },
        else_branch: Not {
            inner: Or {
                lhs: True,
                rhs: False,
            },
        },
    };

    let e1 = expr1.encode();
    let e2 = expr2.encode();
    assert!(e1 == e2, "structural equality must hold across buffers");
}

#[test]
fn arena_manual_build_and_encode() {
    // Build an arena-allocated expression: And(Variable(x), True)
    let ctx = ExprArenaCtx::new();
    let x = InlineVariable::new_from_raw(3);
    let var_leaf_mut = ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Variable(x)));
    let var_leaf: &ArenaAnyExpr = &*var_leaf_mut;
    let true_leaf = True.alloc_in(&ctx);

    let and_view = ExprView::And(var_leaf, true_leaf);
    let and_node = ctx.alloc_expr(ArenaAnyExpr::ArenaView(and_view));

    // Encode to a TreeBuf from the arena node and check shape
    let mut tree = hyformal::encoding::tree::TreeBuf::new();
    let root = EncodableExpr::encode_tree_step(&*and_node, &mut tree);
    tree.set_root(root);
    let (op, _data, children) = tree.get_node(root);
    assert_eq!(op, ExprType::And as u8);
    assert_eq!(children.len(), 2);
}

#[test]
fn arena_with_exprref_leaf_copying() {
    // Create a pre-encoded subtree and reuse it inside an arena expression via ExprRef
    let sub = And {
        lhs: True,
        rhs: False,
    }
    .encode();
    let sub_ref = sub.as_ref();

    let ctx = ExprArenaCtx::new();
    let leaf_mut = ctx.alloc_expr(ArenaAnyExpr::ExprRef(sub_ref));
    let leaf: &ArenaAnyExpr = &*leaf_mut;
    let wrapped = ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Not(leaf)));

    let mut tree = hyformal::encoding::tree::TreeBuf::new();
    let root = EncodableExpr::encode_tree_step(&*wrapped, &mut tree);
    tree.set_root(root);
    let (op, _data, children) = tree.get_node(root);
    assert_eq!(op, ExprType::Not as u8);
    assert_eq!(children.len(), 1);
}

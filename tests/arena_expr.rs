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
    let var_leaf = ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Variable(x)));
    let true_leaf = True.alloc_in(&ctx);

    let and_view = ExprView::And(var_leaf, true_leaf);
    let and_node = ctx.alloc_expr(ArenaAnyExpr::ArenaView(and_view));

    // Encode to a TreeBuf from the arena node and check shape
    let mut tree = hyformal::encoding::tree::TreeBuf::new();
    let root = EncodableExpr::encode_tree_step(and_node, &mut tree);
    tree.set_root(root);
    let (op, _data, children) = tree.get_node(root);
    assert_eq!(op, ExprType::And as u8);
    assert_eq!(children.len(), 2);
}

#[test]
fn deep_copy_simple_leaf_and_unary() {
    let ctx = ExprArenaCtx::new();
    let leaf = True.alloc_in(&ctx);
    let not_leaf = ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Not(leaf)));

    // Deep copy the Not(True) node
    let copy = ctx.deep_copy(not_leaf);

    // Encoding both should yield identical buffers
    let e1 = not_leaf.encode();
    let e2 = copy.encode();
    assert_eq!(type_of(&e1), ExprType::Not);
    assert_eq!(type_of(&e2), ExprType::Not);
    assert!(e1 == e2);
}

#[test]
fn deep_copy_complex_tree_and_independence() {
    // Build: If(And(True, False), Lambda(x, Tuple(x, True))(False), Forall x: Bool. (x = x))
    let ctx = ExprArenaCtx::new();
    let t = True.alloc_in(&ctx);
    let f = False.alloc_in(&ctx);
    let and = ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::And(t, f)));

    let x = InlineVariable::new_from_raw(42);
    let var_x = ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Variable(x)));
    let tup = ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Tuple(var_x, t)));
    let lam = ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Lambda {
        arg: var_x,
        body: tup,
    }));
    let app = ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Call {
        func: lam,
        arg: f,
    }));

    let dtype = Bool.alloc_in(&ctx);
    let var_x2 = ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Variable(x)));
    let eq_x = ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Equal(var_x2, var_x2)));
    let forall = ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Forall {
        variable: x,
        dtype,
        inner: eq_x,
    }));

    let root = ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::If {
        condition: and,
        then_branch: app,
        else_branch: forall,
    }));

    // Deep copy
    let copy = ctx.deep_copy(root);

    // Structural equality after encoding
    let e1 = root.encode();
    let e2 = copy.encode();
    assert!(e1 == e2);

    // Mutate original root to ensure independence: wrap with Not
    let wrapped_original = ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Not(root)));
    let e_wrapped = wrapped_original.encode();
    assert_eq!(type_of(&e_wrapped), ExprType::Not);

    // Copy should remain the same as before
    let e2_again = copy.encode();
    assert!(e2 == e2_again);
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
    let leaf = ctx.alloc_expr(ArenaAnyExpr::ExprRef(sub_ref));
    let wrapped = ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Not(leaf)));

    let mut tree = hyformal::encoding::tree::TreeBuf::new();
    let root = EncodableExpr::encode_tree_step(wrapped, &mut tree);
    tree.set_root(root);
    let (op, _data, children) = tree.get_node(root);
    assert_eq!(op, ExprType::Not as u8);
    assert_eq!(children.len(), 1);
}

#[test]
fn deep_copy_ref_simple_and_unary() {
    // Build a small owned expression and import it into an arena
    let owned = Not { inner: True }.encode();
    let borrowed = owned.as_ref();

    let ctx = ExprArenaCtx::new();
    let copied = ctx.deep_copy_ref(borrowed);

    // Copied arena tree should encode identically
    let copied_encoded = copied.encode();
    assert!(owned == copied_encoded);
    assert_eq!(copied.view().type_(), ExprType::Not);

    // Wrap the copy and ensure shape
    let wrapped = ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Not(copied)));
    let wrapped_encoded = wrapped.encode();
    assert_eq!(wrapped_encoded.as_ref().type_(), ExprType::Not);
}

#[test]
fn deep_copy_ref_complex_mixed_variants() {
    // Construct a complex owned expression using defs, then import into arena and compare.
    // expr := if (and(true, false)) then (lambda x. (x, true))(false) else forall x: Bool. (x = x)
    let x = InlineVariable::new_from_raw(7);
    let owned_expr = If {
        condition: And {
            lhs: True,
            rhs: False,
        },
        then_branch: Call {
            func: Lambda {
                arg: InlineVariable::new_from_raw(7),
                body: Tuple {
                    first: InlineVariable::new_from_raw(7),
                    second: True,
                },
            },
            arg: False,
        },
        else_branch: ForAll {
            variable: x,
            dtype: Bool,
            inner: Equal {
                lhs: InlineVariable::new_from_raw(7),
                rhs: InlineVariable::new_from_raw(7),
            },
        },
    }
    .encode();

    let ctx = ExprArenaCtx::new();
    let copied = ctx.deep_copy_ref(owned_expr.as_ref());

    // Verify structural equality via encoding
    let copied_encoded = copied.encode();
    assert!(
        owned_expr == copied_encoded,
        "Expected equality:\n{}\n  !=\n{}",
        owned_expr,
        copied_encoded
    );

    // Perform additional manipulations on arena nodes to check independence
    let negated = ctx.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::Not(copied)));
    let neg_encoded = negated.encode();
    assert_eq!(neg_encoded.as_ref().type_(), ExprType::Not);

    // Original owned buffer remains the same
    let owned_again = owned_expr.as_ref().encode();
    assert!(
        owned_again == owned_expr,
        "Original owned expression was mutated:\n{}\n  !=\n{}",
        owned_again,
        owned_expr
    );
}

#[test]
fn deep_copy_ref_handles_nested_exprref_and_views() {
    // Start with an owned subtree and bring it as ExprRef into an arena structure,
    // then deep_copy_ref on the whole encoded result to ensure it re-materializes as ArenaView nodes.
    let sub_owned = And {
        lhs: True,
        rhs: False,
    }
    .encode();
    let sub_ref = sub_owned.as_ref();

    let ctx1 = ExprArenaCtx::new();
    let leaf = ctx1.alloc_expr(ArenaAnyExpr::ExprRef(sub_ref));
    let wrapped = ctx1.alloc_expr(ArenaAnyExpr::ArenaView(ExprView::If {
        condition: leaf,
        then_branch: True.alloc_in(&ctx1),
        else_branch: False.alloc_in(&ctx1),
    }));

    // Encode the mixed arena expression (contains an ExprRef leaf)
    let mixed_encoded = wrapped.encode();

    // Now deep-copy-from-ref into a fresh arena; this should produce only ArenaView nodes
    let ctx2 = ExprArenaCtx::new();
    let re_mat = ctx2.deep_copy_ref(mixed_encoded.as_ref());
    let re_mat_view = re_mat.view();
    assert_eq!(re_mat_view.type_(), ExprType::If);

    // Structural equality preserved across the transformation
    assert!(mixed_encoded == re_mat.encode());
}

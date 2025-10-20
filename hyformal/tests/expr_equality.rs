use hyformal::expr::defs::*;
use hyformal::expr::func;
use hyformal::expr::view::ExprView;
use hyformal::expr::{AnyExpr, Expr};
use hyformal::variable::{InlineVariable, Variable};

fn v_internal(id: u32) -> InlineVariable {
    Variable::Internal(id).into()
}
fn v_external(id: u32) -> InlineVariable {
    Variable::External(id).into()
}

#[test]
fn anyexpr_structural_equality_across_buffers() {
    let a1: AnyExpr = (True & False).encode();
    let a2: AnyExpr = (True & False).encode();
    assert!(a1 == a2);
    assert!(a1.as_ref() == a2.as_ref());

    let b1: AnyExpr = (True | False).encode();
    assert!(a1 != b1);
    assert!(a1.as_ref() != b1.as_ref());
}

#[test]
fn anyexprref_same_buffer_different_nodes_are_not_equal() {
    let x = v_internal(0);
    // Lambda where children are different: arg = x, body = True
    let lam: AnyExpr = x.lambda(True).encode();
    let lam_ref = lam.as_ref();
    if let ExprView::Lambda { arg, body } = lam_ref.view() {
        // Compare by encoding to circumvent opaque impl Trait: arg != body
        let arg_e: AnyExpr = arg.encode();
        let body_e: AnyExpr = body.encode();
        assert!(
            arg_e != body_e,
            "Different children in same buffer must not be equal"
        );
        // Sanity: same child equals itself
        let arg2_e: AnyExpr = arg.encode();
        assert!(arg_e == arg2_e);
    } else {
        panic!("expected lambda view");
    }
}

#[test]
fn anyexprref_equal_subtrees_across_buffers() {
    // Build two identical lambdas independently
    let x1 = v_internal(1);
    let e1: AnyExpr = x1.lambda(x1).encode();

    let x2 = v_internal(1);
    let e2: AnyExpr = x2.lambda(x2).encode();

    assert!(e1.as_ref() == e2.as_ref());
}

#[test]
fn variable_payload_affects_equality() {
    let xi = v_internal(2);
    let xe = v_external(2);

    let vi: AnyExpr = xi.encode();
    let ve: AnyExpr = xe.encode();
    assert!(
        vi != ve,
        "internal vs external variables with same id must differ"
    );

    // Equality nodes on top should still differ
    let e_vi: AnyExpr = xi.equals(xi).encode();
    let e_ve: AnyExpr = xe.equals(xe).encode();
    assert!(e_vi != e_ve);
}

#[test]
fn quantifier_equality_and_inequality() {
    let x = v_external(3);
    let q1: AnyExpr = func::forall(x, Bool, True).encode();
    let q2: AnyExpr = func::forall(x, Bool, True).encode();
    assert!(q1 == q2);

    // Change bound variable id => not equal
    let y = v_external(4);
    let q3: AnyExpr = func::forall(y, Bool, True).encode();
    assert!(q1 != q3);

    // Change dtype or body => not equal
    let q4: AnyExpr = func::forall(x, Omega, True).encode();
    assert!(q1 != q4);
    let q5: AnyExpr = func::forall(x, Bool, False).encode();
    assert!(q1 != q5);
}

#[test]
fn deep_structural_equality() {
    // if (True) then (x = x) else (tuple(Bool, Omega))
    let x1 = v_internal(5);
    let e1: AnyExpr = func::branch(True, x1.equals(x1), func::tuple(Bool, Omega)).encode();
    let x2 = v_internal(5);
    let e2: AnyExpr = func::branch(True, x2.equals(x2), func::tuple(Bool, Omega)).encode();
    assert!(e1 == e2);
}

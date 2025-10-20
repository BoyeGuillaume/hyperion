use hyformal::expr::Expr;
use hyformal::expr::defs::*;
use hyformal::expr::variant::ExprType;
use hyformal::variable::InlineVariable;

#[test]
fn lib_rs_doc_example_compiles_and_behaves() {
    // Types as expressions: (Bool -> Bool) x Bool
    let ty = Bool.lambda(Bool).tuple(Bool);
    let dyn_ty = ty.encode();
    assert_eq!(dyn_ty.as_ref().view().type_(), ExprType::Tuple);

    // Terms and logic as expressions
    let f = InlineVariable::new_from_raw(0);
    let x = InlineVariable::new_from_raw(1);
    let app = f.apply(x);
    let eq = app.equals(x);
    let quantified = ForAll {
        variable: x,
        dtype: Bool,
        inner: eq,
    };
    let dyn_e = quantified.encode();
    let er = dyn_e.as_ref();
    let view = er.view();
    assert!(matches!(
        view,
        hyformal::expr::view::ExprView::Forall { .. }
    ));
}

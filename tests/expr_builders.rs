use hyformal::expr::defs::*;
use hyformal::expr::variant::ExprType;
use hyformal::expr::{AnyExpr, Expr};
use hyformal::expr::{func, view::ExprView};
use hyformal::variable::InlineVariable;

fn view_type<E: Expr>(e: &E) -> ExprType {
    e.view().type_()
}

#[test]
fn simple_builders_types_and_terms() {
    // Type constructors
    assert_eq!(view_type(&Bool), ExprType::Bool);
    assert_eq!(view_type(&Omega), ExprType::Omega);
    assert_eq!(view_type(&Never), ExprType::Never);

    // Unary and binary builders using trait methods
    let t = Bool.powerset();
    assert_eq!(view_type(&t), ExprType::Powerset);

    let a = True & False; // And
    assert_eq!(view_type(&a), ExprType::And);

    let o = True | False; // Or
    assert_eq!(view_type(&o), ExprType::Or);

    let n = !True; // Not
    assert_eq!(view_type(&n), ExprType::Not);

    let i = Implies {
        antecedent: True,
        consequent: False,
    };
    assert_eq!(view_type(&i), ExprType::Implies);

    let iff = Iff {
        lhs: True,
        rhs: False,
    };
    assert_eq!(view_type(&iff), ExprType::Iff);
}

#[test]
fn term_level_builders_and_encode_decode() {
    let f = InlineVariable::new_from_raw(0);
    let x = InlineVariable::new_from_raw(1);
    let lam = f.lambda(f.apply(x));
    assert_eq!(view_type(&lam), ExprType::Lambda);

    // Encode to AnyExpr and borrow back
    let encoded: AnyExpr = lam.encode();
    let borrowed = encoded.as_ref();
    match borrowed.view() {
        ExprView::Lambda { arg, body } => {
            assert!(matches!(arg.view(), ExprView::Variable(v) if v == f));
            assert!(
                matches!(body.view(), ExprView::Call { func, arg } if matches!(func.view(), ExprView::Variable(v) if v == f) && matches!(arg.view(), ExprView::Variable(v) if v == x))
            );
        }
        _ => panic!("expected lambda"),
    }
}

#[test]
fn tuple_equals_helpers_and_free_functions() {
    let a = True.tuple(False);
    assert_eq!(view_type(&a), ExprType::Tuple);

    let eq = a.equals(a);
    assert_eq!(view_type(&eq), ExprType::Equal);

    // Free-function builders mirror trait methods
    let f = InlineVariable::new_from_raw(2);
    let call = func::call(f, True);
    assert_eq!(view_type(&call), ExprType::Call);

    let lam = func::lambda(f, False);
    assert_eq!(view_type(&lam), ExprType::Lambda);

    let pow = func::powerset(Bool);
    assert_eq!(view_type(&pow), ExprType::Powerset);

    let anded = func::and(True, False);
    assert_eq!(view_type(&anded), ExprType::And);

    let ored = func::or(True, False);
    assert_eq!(view_type(&ored), ExprType::Or);

    let iff = func::iff(True, False);
    assert_eq!(view_type(&iff), ExprType::Iff);

    let not = func::not(True);
    assert_eq!(view_type(&not), ExprType::Not);
}

#[test]
fn quantifiers_and_complex_structure() {
    let x = InlineVariable::new_from_raw(5);
    let prop = (True & !False) | (True & True) & (!!True | !False);
    let ty = Bool.powerset().tuple(Omega);
    let forall = ForAll {
        variable: x,
        dtype: ty,
        inner: prop.equals(True),
    };
    assert_eq!(view_type(&forall), ExprType::Forall);

    let exists = Exists {
        variable: x,
        dtype: Bool,
        inner: True,
    };
    assert_eq!(view_type(&exists), ExprType::Exists);

    // Deep encode/decode and verify children counts via pattern matching
    let any = forall.encode();
    match any.as_ref().view() {
        ExprView::Forall {
            variable,
            dtype,
            inner,
        } => {
            assert_eq!(variable, x);
            assert!(matches!(dtype.view(), ExprView::Tuple(_, _)));
            assert!(matches!(inner.view(), ExprView::Equal(_, _)));
        }
        _ => panic!("expected forall"),
    }
}

#[test]
fn consolidation_does_not_change_semantics() {
    let x = InlineVariable::new_from_raw(7);
    let e = x.lambda((x.apply(True)).equals(False)).encode();
    let mut e2 = e.clone();
    // Make sure consolidate is callable and keeps view identical
    e2.consolidate();
    assert_eq!(e.as_ref().view().type_(), e2.as_ref().view().type_());
}

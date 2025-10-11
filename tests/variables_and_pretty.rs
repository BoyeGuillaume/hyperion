use hyformal::expr::Expr;
use hyformal::expr::defs::*;
use hyformal::expr::pretty::PrettyExpr;
use hyformal::variable::{InlineVariable, Variable};

#[test]
fn inline_variable_encoding_and_display() {
    let v = Variable::Internal(3);
    let iv: InlineVariable = v.into();
    assert_eq!(iv.to_variable(), v);

    // Small ids print as a..z, larger as v<N>
    let small = InlineVariable::new_from_raw(0);
    assert_eq!(format!("{small}"), "a");
    let large = InlineVariable::new_from_raw(26);
    assert_eq!(format!("{large}"), "v0");
}

#[test]
fn pretty_printer_respects_precedence() {
    // Check parentheses insertion for equality vs and
    let a = True.equals(False & True);
    let s = a.pretty_string();
    assert_eq!(s, "true = (false /\\ true)");

    // Lambda with body needing parens when printed as part of a larger expression
    let x = InlineVariable::new_from_raw(1);
    let lam = x.lambda(True & False);
    let call = lam.apply(True);
    let sc = call.pretty_string();
    // Expect: (x -> true /\ false)(true)
    assert!(sc.contains("->"));
    assert!(sc.contains("(true)"));
}

#[test]
fn pretty_print_basic_nodes() {
    let s_bool = Bool.pretty_string();
    let s_ps = Powerset { inner: Bool }.pretty_string();
    assert_eq!(s_bool, "Bool");
    assert_eq!(s_ps, "Powerset(Bool)");
}

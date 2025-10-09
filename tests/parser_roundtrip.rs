use hyformal::expr::Expr;
use hyformal::expr::pretty::PrettyExpr;
use hyformal::expr::variant::ExprType;
use hyformal::parser::parse;

fn roundtrip(src: &str) -> String {
    let e = parse(src).expect("parse should succeed");
    e.as_ref().pretty_string()
}

#[test]
fn parse_simple_atoms() {
    assert_eq!(roundtrip("true"), "true");
    assert_eq!(roundtrip("false"), "false");
    assert_eq!(roundtrip("Bool"), "Bool");
    assert_eq!(roundtrip("Omega"), "Omega");
    assert_eq!(roundtrip("<>"), "<>");
}

#[test]
fn precedence_and_associativity() {
    // Not higher than and/or
    assert_eq!(roundtrip("!a /\\ b"), "!a /\\ b");
    assert_eq!(roundtrip("a \\/ !b"), "a \\/ !b");

    // Equality binds tighter than and/or/iff/implies
    assert_eq!(roundtrip("a = b /\\ c"), "a = b /\\ c");
    assert_eq!(roundtrip("a /\\ b = c"), "a /\\ b = c");

    // Implies right-assoc
    assert_eq!(roundtrip("a => b => c"), "a => (b => c)");
    assert_eq!(roundtrip("(a => b) => c"), "(a => b) => c");

    // Tuples left-assoc
    assert_eq!(roundtrip("a, b, c"), "a, b, c");

    // Calls left-assoc; pretty-printer parenthesizes nested function position
    assert_eq!(roundtrip("f(a)(b)"), "(f(a))(b)");
}

#[test]
fn quantifiers_and_if() {
    let s = "forall a : Bool . exists b : Powerset(Bool) . if a then b(a) else <>";
    let pretty = roundtrip(s);
    // Ensure top-level constructs preserved; exact spacing may differ but keywords and structure remain
    assert!(pretty.contains("forall"));
    assert!(pretty.contains("exists"));
    assert!(pretty.contains("if"));
}

#[test]
fn variables_lexing() {
    // single-letter and v<number> forms
    assert_eq!(roundtrip("a"), "a");
    assert_eq!(roundtrip("A"), "a"); // Display lowers by default
    // v0 corresponds to raw id 26
    let s = roundtrip("v0");
    assert_eq!(s, "v0");
}

#[test]
fn parse_errors_are_reported() {
    let err = parse("forall : .").err().expect("expected parse error");
    assert!(!err.is_empty());
    assert!(
        err.iter()
            .any(|e| e.contains("parse error") || e.contains("lexing error"))
    );
}

#[test]
fn large_complex_expression() {
    // Build a relatively complex expression with different operators and nesting
    let src = "forall x : Bool . (x => true) <=> (!x \\/ false) /\\ (x = x)";
    let e = parse(src).expect("parse ok");
    let v = e.as_ref().view().type_();
    assert_eq!(v, ExprType::Forall);
}

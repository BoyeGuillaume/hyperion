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
    assert_eq!(roundtrip("!%1 /\\ %2"), "!%1 /\\ %2");
    assert_eq!(roundtrip("%1 \\/ !%2"), "%1 \\/ !%2");

    // Equality binds tighter than and/or/iff/implies
    assert_eq!(roundtrip("%1 = %2 /\\ %3"), "%1 = %2 /\\ %3");
    assert_eq!(roundtrip("%1 /\\ %2 = %3"), "%1 /\\ %2 = %3");

    // Implies right-assoc
    assert_eq!(roundtrip("%1 => %2 => %3"), "%1 => (%2 => %3)");
    assert_eq!(roundtrip("(%1 => %2) => %3"), "(%1 => %2) => %3");

    // Tuples left-assoc
    assert_eq!(roundtrip("$000001, %00002, %3"), "$1, %2, %3");

    // Calls left-assoc; pretty-printer parenthesizes nested function position
    assert_eq!(roundtrip("$00001(%0001)($00002)"), "($1(%1))($2)");
}

#[test]
fn quantifiers_and_if() {
    let s = "forall $0 : Bool . exists $0 : Powerset(Bool) . if $0 then $1($0) else <>";
    let pretty = roundtrip(s);
    // Ensure top-level constructs preserved; exact spacing may differ but keywords and structure remain
    assert!(pretty.contains("forall"));
    assert!(pretty.contains("exists"));
    assert!(pretty.contains("if"));
}

#[test]
fn variables_lexing() {
    // single-letter and v<number> forms
    assert_eq!(roundtrip("$0"), "$0");
    assert_eq!(roundtrip("%0000A"), "%a"); // Display lowers by default
    let s = roundtrip("%00000");
    assert_eq!(s, "%0");
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
    let src = "forall %1 : Bool . (%1 => true) <=> (!%1 \\/ false) /\\ (%1 = %1)";
    let e = parse(src).expect("parse ok");
    let v = e.as_ref().view().type_();
    assert_eq!(v, ExprType::Forall);
}

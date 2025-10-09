use hyformal::prelude::*;

fn main() {
    let a = InlineVariable::new_from_raw(0);
    let b = InlineVariable::new_from_raw(1);
    let c = InlineVariable::new_from_raw(2);

    let expr = forall(
        a,
        Bool,
        implies(
            a,
            exists(
                b,
                powerset(tuple(Omega, tuple(Bool, Bool)).lambda(Never)),
                and(and(a, equals(c, c)), equals(not(a) | c, b)),
            ),
        ),
    );
    expr.pretty_print().unwrap();
    println!();
}

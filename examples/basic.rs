use hyformal::expr::defs::*;
use hyformal::expr::*;
use hyformal::variable::InlineVariable;

fn main() {
    let a = InlineVariable::new_from_raw(0);
    let b = InlineVariable::new_from_raw(1);

    let expr = forall(a, Bool, implies(a, exists(b, Bool, and(a, b))));
    expr.pretty_print().unwrap();
}

use strum::{EnumDiscriminants, EnumIs};

use crate::{expr::Expr, prop::Prop, variable::InlineVariable};

#[derive(Debug, Clone, Copy, EnumIs, EnumDiscriminants)]
#[strum_discriminants(derive(PartialOrd, Ord, Hash))]
#[strum_discriminants(name(ExprDispatchVariant))]
pub enum ExprDispatch<P: Prop, T1: Expr, T2: Expr> {
    Var(InlineVariable),
    Unreachable,
    App {
        func: InlineVariable,
        arg: T1,
    },
    If {
        condition: P,
        then_branch: T1,
        else_branch: T2,
    },
    Tuple(T1, T2),
    Prop(P),
}

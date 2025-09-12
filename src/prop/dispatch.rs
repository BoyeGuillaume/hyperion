use strum::{EnumDiscriminants, EnumIs};

use crate::{dtype::DType, prop::Prop, term::Term, variable::InlineVariable};

#[derive(Debug, Clone, Copy, EnumIs, EnumDiscriminants)]
#[strum_discriminants(derive(PartialOrd, Ord, Hash))]
#[strum_discriminants(name(PropDispatchVariant))]
pub enum PropDispatch<P1: Prop, P2: Prop, T1: Term, T2: Term, DT: DType> {
    True,
    False,
    Not(P1),
    And(P1, P2),
    Or(P1, P2),
    Implies(P1, P2),
    Iff(P1, P2),
    ForAll {
        variable: InlineVariable,
        dtype: DT,
        inner: P1,
    },
    Exists {
        variable: InlineVariable,
        dtype: DT,
        inner: P1,
    },
    Equal(T1, T2),
}

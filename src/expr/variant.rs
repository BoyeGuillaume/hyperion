use strum::EnumIter;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter)]
#[repr(u8)]
pub enum ExprType {
    // Constant expr
    Bool,
    Omega,
    True,
    False,
    Never,

    // Unary expr
    Not,
    Powerset,

    // Binary expr
    And,
    Or,
    Implies,
    Iff,
    Equal,
    Lambda,
    Tuple,
    Forall,
    Exists,

    // Ternary expr
    If,

    // Misc
    Var,
}

//! Concrete unified expression constructors: terms, logic, and types.
//!
//! All these types implement [`crate::expr::Expr`] and support encoding/decoding.
use crate::{
    encoding::{
        EncodableExpr,
        tree::{TreeBuf, TreeBufNodeRef},
    },
    expr::{AnyExpr, Expr, expr_sealed, variant::ExprType, view::ExprView},
    variable::InlineVariable,
};

// Lightweight operator sugar for logical combinations on expressions.
macro_rules! define_ops_expr {
    (
        $name:ident
        $( <
            $( $($lft:lifetime),+ $(,)? )?
            $( $($gen_name:ident: $gen:tt ),+ $(,)? )?
        > )?
    ) => {
        impl <
            $(
                $( $( $lft ),+ , )?
                $( $( $gen_name: $gen ),+ , )?
            )?
            _O1: Expr
        > std::ops::BitAnd<_O1> for $name $( <
                $( $( $lft ),+ , )?
                $( $( $gen_name ),* )?
            > )? {
            type Output = And<Self, _O1>;

            fn bitand(self, rhs: _O1) -> Self::Output {
                And { lhs: self, rhs: rhs }
            }
        }

        impl <
            $(
                $( $( $lft ),+ , )?
                $( $( $gen_name: $gen ),+ , )?
            )?
            _O1: Expr
        > std::ops::BitOr<_O1> for $name $( <
                $( $( $lft ),+ , )?
                $( $( $gen_name ),* )?
            > )? {
            type Output = Or<Self, _O1>;

            fn bitor(self, rhs: _O1) -> Self::Output {
                Or { lhs: self, rhs: rhs }
            }
        }

        impl <
            $(
                $( $( $lft ),+ , )?
                $( $( $gen_name: $gen ),+ , )?
            )?
        > std::ops::Not for $name $( <
                $( $( $lft ),+ , )?
                $( $( $gen_name ),* )?
            > )? {
            type Output = Not<Self>;

            fn not(self) -> Self::Output {
                Not { inner: self }
            }
        }
    };
}

// ========================= Propositional variables =========================
// True.
#[derive(Clone, Copy)]
pub struct True;

impl expr_sealed::Sealed for True {}

impl EncodableExpr for True {
    fn encode_tree_step(self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        tree.push_node(ExprType::True as u8, None, &[])
    }
}

impl Expr for True {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<AnyExpr, AnyExpr, AnyExpr>::True
    }
}

define_ops_expr! { True }

// False.
#[derive(Clone, Copy)]
pub struct False;

impl expr_sealed::Sealed for False {}

impl EncodableExpr for False {
    fn encode_tree_step(self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        tree.push_node(ExprType::False as u8, None, &[])
    }
}

impl Expr for False {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<AnyExpr, AnyExpr, AnyExpr>::False
    }
}

define_ops_expr! { False }

// Not.
#[derive(Clone, Copy)]
pub struct Not<P: Expr> {
    pub inner: P,
}

impl<P: Expr> expr_sealed::Sealed for Not<P> {}

impl<P: Expr> EncodableExpr for Not<P> {
    fn encode_tree_step(self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        let inner_ref = self.inner.encode_tree_step(tree);
        tree.push_node(ExprType::Not as u8, None, &[inner_ref])
    }
}

impl<P: Expr> Expr for Not<P> {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<&P, AnyExpr, AnyExpr>::Not(&self.inner)
    }
}

define_ops_expr! { Not<P: Expr> }

// And.
#[derive(Clone, Copy)]
pub struct And<P: Expr, Q: Expr> {
    pub lhs: P,
    pub rhs: Q,
}

impl<P: Expr, Q: Expr> expr_sealed::Sealed for And<P, Q> {}

impl<P: Expr, Q: Expr> EncodableExpr for And<P, Q> {
    fn encode_tree_step(self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        let left_ref = self.lhs.encode_tree_step(tree);
        let right_ref = self.rhs.encode_tree_step(tree);
        tree.push_node(ExprType::And as u8, None, &[left_ref, right_ref])
    }
}

impl<P: Expr, Q: Expr> Expr for And<P, Q> {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<&P, &Q, AnyExpr>::And(&self.lhs, &self.rhs)
    }
}

define_ops_expr! { And<P: Expr, Q: Expr> }

// Or.
#[derive(Clone, Copy)]
pub struct Or<P: Expr, Q: Expr> {
    pub lhs: P,
    pub rhs: Q,
}

impl<P: Expr, Q: Expr> expr_sealed::Sealed for Or<P, Q> {}

impl<P: Expr, Q: Expr> EncodableExpr for Or<P, Q> {
    fn encode_tree_step(self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        let left_ref = self.lhs.encode_tree_step(tree);
        let right_ref = self.rhs.encode_tree_step(tree);
        tree.push_node(ExprType::Or as u8, None, &[left_ref, right_ref])
    }
}

impl<P: Expr, Q: Expr> Expr for Or<P, Q> {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<&P, &Q, AnyExpr>::Or(&self.lhs, &self.rhs)
    }
}

define_ops_expr! { Or<P: Expr, Q: Expr> }

// Implies.
#[derive(Clone, Copy)]
pub struct Implies<P: Expr, Q: Expr> {
    pub antecedent: P,
    pub consequent: Q,
}

impl<P: Expr, Q: Expr> expr_sealed::Sealed for Implies<P, Q> {}

impl<P: Expr, Q: Expr> EncodableExpr for Implies<P, Q> {
    fn encode_tree_step(self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        let left_ref = self.antecedent.encode_tree_step(tree);
        let right_ref = self.consequent.encode_tree_step(tree);
        tree.push_node(ExprType::Implies as u8, None, &[left_ref, right_ref])
    }
}

impl<P: Expr, Q: Expr> Expr for Implies<P, Q> {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<&P, &Q, AnyExpr>::Implies(&self.antecedent, &self.consequent)
    }
}

define_ops_expr! { Implies<P: Expr, Q: Expr> }

// Iff.
#[derive(Clone, Copy)]
pub struct Iff<P: Expr, Q: Expr> {
    pub lhs: P,
    pub rhs: Q,
}

impl<P: Expr, Q: Expr> expr_sealed::Sealed for Iff<P, Q> {}

impl<P: Expr, Q: Expr> EncodableExpr for Iff<P, Q> {
    fn encode_tree_step(self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        let left_ref = self.lhs.encode_tree_step(tree);
        let right_ref = self.rhs.encode_tree_step(tree);
        tree.push_node(ExprType::Iff as u8, None, &[left_ref, right_ref])
    }
}

impl<P: Expr, Q: Expr> Expr for Iff<P, Q> {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<&P, &Q, AnyExpr>::Iff(&self.lhs, &self.rhs)
    }
}

define_ops_expr! { Iff<P: Expr, Q: Expr> }

#[derive(Clone, Copy)]
pub struct Eq<P: Expr, Q: Expr> {
    pub lhs: P,
    pub rhs: Q,
}

impl<P: Expr, Q: Expr> expr_sealed::Sealed for Eq<P, Q> {}

impl<P: Expr, Q: Expr> EncodableExpr for Eq<P, Q> {
    fn encode_tree_step(self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        let left_ref = self.lhs.encode_tree_step(tree);
        let right_ref = self.rhs.encode_tree_step(tree);
        tree.push_node(ExprType::Equal as u8, None, &[left_ref, right_ref])
    }
}

impl<P: Expr, Q: Expr> Expr for Eq<P, Q> {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<&P, &Q, AnyExpr>::Equal(&self.lhs, &self.rhs)
    }
}

define_ops_expr! { Eq<P: Expr, Q: Expr> }

// ======================== Quantified variables =========================
// Forall.
#[derive(Clone, Copy)]
pub struct ForAll<DT: Expr, P: Expr> {
    pub variable: InlineVariable,
    pub dtype: DT,
    pub inner: P,
}

impl<DT: Expr, P: Expr> expr_sealed::Sealed for ForAll<DT, P> {}

impl<DT: Expr, P: Expr> EncodableExpr for ForAll<DT, P> {
    fn encode_tree_step(self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        let dtype_ref = self.dtype.encode_tree_step(tree);
        let inner_ref = self.inner.encode_tree_step(tree);
        tree.push_node(
            ExprType::Forall as u8,
            Some(self.variable.raw()),
            &[dtype_ref, inner_ref],
        )
    }
}

impl<DT: Expr, P: Expr> Expr for ForAll<DT, P> {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<&DT, &P, AnyExpr>::Forall {
            variable: self.variable,
            dtype: &self.dtype,
            inner: &self.inner,
        }
    }
}

define_ops_expr! { ForAll<DT: Expr, P: Expr> }

// Exists.
#[derive(Clone, Copy)]
pub struct Exists<DT: Expr, P: Expr> {
    pub variable: InlineVariable,
    pub dtype: DT,
    pub inner: P,
}

impl<DT: Expr, P: Expr> expr_sealed::Sealed for Exists<DT, P> {}

impl<DT: Expr, P: Expr> EncodableExpr for Exists<DT, P> {
    fn encode_tree_step(self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        let dtype_ref = self.dtype.encode_tree_step(tree);
        let inner_ref = self.inner.encode_tree_step(tree);
        tree.push_node(
            ExprType::Exists as u8,
            Some(self.variable.raw()),
            &[dtype_ref, inner_ref],
        )
    }
}

impl<DT: Expr, P: Expr> Expr for Exists<DT, P> {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<&DT, &P, AnyExpr>::Exists {
            variable: self.variable,
            dtype: &self.dtype,
            inner: &self.inner,
        }
    }
}

define_ops_expr! { Exists<DT: Expr, P: Expr> }

// ========================= Other expressions (not logic) =========================
// ========================= Constants =========================

// Bool.
#[derive(Clone, Copy)]
pub struct Bool;

impl expr_sealed::Sealed for Bool {}

impl EncodableExpr for Bool {
    fn encode_tree_step(self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        tree.push_node(ExprType::Bool as u8, None, &[])
    }
}

impl Expr for Bool {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<AnyExpr, AnyExpr, AnyExpr>::Bool
    }
}

// Omega.
#[derive(Clone, Copy)]
pub struct Omega;

impl expr_sealed::Sealed for Omega {}

impl EncodableExpr for Omega {
    fn encode_tree_step(self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        tree.push_node(ExprType::Omega as u8, None, &[])
    }
}

impl Expr for Omega {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<AnyExpr, AnyExpr, AnyExpr>::Omega
    }
}

// Never.
#[derive(Clone, Copy)]
pub struct Never;

impl expr_sealed::Sealed for Never {}

impl EncodableExpr for Never {
    fn encode_tree_step(self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        tree.push_node(ExprType::Never as u8, None, &[])
    }
}

impl Expr for Never {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<AnyExpr, AnyExpr, AnyExpr>::Never
    }
}

// ========================= Power set =========================
#[derive(Clone, Copy)]
pub struct PowerSet<P: Expr> {
    pub inner: P,
}

impl<P: Expr> expr_sealed::Sealed for PowerSet<P> {}

impl<P: Expr> EncodableExpr for PowerSet<P> {
    fn encode_tree_step(self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        let inner_ref = self.inner.encode_tree_step(tree);
        tree.push_node(ExprType::Powerset as u8, None, &[inner_ref])
    }
}

impl<P: Expr> Expr for PowerSet<P> {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<&P, AnyExpr, AnyExpr>::Powerset(&self.inner)
    }
}

// ======================== Other binary expressions =========================
// Lambda.
#[derive(Clone, Copy)]
pub struct Lambda<A: Expr, B: Expr> {
    pub arg: A,
    pub body: B,
}

impl<A: Expr, B: Expr> expr_sealed::Sealed for Lambda<A, B> {}

impl<A: Expr, B: Expr> EncodableExpr for Lambda<A, B> {
    fn encode_tree_step(self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        let arg_ref = self.arg.encode_tree_step(tree);
        let body_ref = self.body.encode_tree_step(tree);
        tree.push_node(ExprType::Lambda as u8, None, &[arg_ref, body_ref])
    }
}

impl<A: Expr, B: Expr> Expr for Lambda<A, B> {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<&A, &B, AnyExpr>::Lambda {
            arg: &self.arg,
            body: &self.body,
        }
    }
}

// Call.
#[derive(Clone, Copy)]
pub struct Call<A: Expr, B: Expr> {
    pub func: A,
    pub arg: B,
}

impl<A: Expr, B: Expr> expr_sealed::Sealed for Call<A, B> {}

impl<A: Expr, B: Expr> EncodableExpr for Call<A, B> {
    fn encode_tree_step(self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        let func_ref = self.func.encode_tree_step(tree);
        let arg_ref = self.arg.encode_tree_step(tree);
        tree.push_node(ExprType::Call as u8, None, &[func_ref, arg_ref])
    }
}

impl<A: Expr, B: Expr> Expr for Call<A, B> {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<&A, &B, AnyExpr>::Call {
            func: &self.func,
            arg: &self.arg,
        }
    }
}

// Tuple.
#[derive(Clone, Copy)]
pub struct Tuple<A: Expr, B: Expr> {
    pub first: A,
    pub second: B,
}

impl<A: Expr, B: Expr> expr_sealed::Sealed for Tuple<A, B> {}

impl<A: Expr, B: Expr> EncodableExpr for Tuple<A, B> {
    fn encode_tree_step(self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        let first_ref = self.first.encode_tree_step(tree);
        let second_ref = self.second.encode_tree_step(tree);
        tree.push_node(ExprType::Tuple as u8, None, &[first_ref, second_ref])
    }
}

impl<A: Expr, B: Expr> Expr for Tuple<A, B> {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<&A, &B, AnyExpr>::Tuple(&self.first, &self.second)
    }
}

// ======================== If-then-else =========================
#[derive(Clone, Copy)]
pub struct If<P: Expr, T: Expr, E: Expr> {
    pub condition: P,
    pub then_branch: T,
    pub else_branch: E,
}

impl<P: Expr, T: Expr, E: Expr> expr_sealed::Sealed for If<P, T, E> {}

impl<P: Expr, T: Expr, E: Expr> EncodableExpr for If<P, T, E> {
    fn encode_tree_step(self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        let cond_ref = self.condition.encode_tree_step(tree);
        let then_ref = self.then_branch.encode_tree_step(tree);
        let else_ref = self.else_branch.encode_tree_step(tree);
        tree.push_node(ExprType::If as u8, None, &[cond_ref, then_ref, else_ref])
    }
}

impl<P: Expr, T: Expr, E: Expr> Expr for If<P, T, E> {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        ExprView::<&P, &T, &E>::If {
            condition: &self.condition,
            then_branch: &self.then_branch,
            else_branch: &self.else_branch,
        }
    }
}

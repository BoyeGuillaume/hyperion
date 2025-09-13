pub mod defs;
pub mod dispatch;

pub(crate) mod expr_sealed {
    pub trait Sealed {}

    impl<'a, T: Sealed> Sealed for &'a T {}
}

pub trait Expr: expr_sealed::Sealed + Sized {
    fn decode_expr(
        &self,
    ) -> crate::expr::dispatch::ExprDispatch<
        impl crate::prop::Prop,
        impl crate::expr::Expr,
        impl crate::expr::Expr,
    >;
}

impl<'a, T: Expr> Expr for &'a T {
    fn decode_expr(
        &self,
    ) -> crate::expr::dispatch::ExprDispatch<
        impl crate::prop::Prop,
        impl crate::expr::Expr,
        impl crate::expr::Expr,
    > {
        (*self).decode_expr()
    }
}

pub struct DynExpr {}
impl expr_sealed::Sealed for DynExpr {}
impl Expr for DynExpr {
    fn decode_expr(
        &self,
    ) -> crate::expr::dispatch::ExprDispatch<
        impl crate::prop::Prop,
        impl crate::expr::Expr,
        impl crate::expr::Expr,
    > {
        crate::expr::dispatch::ExprDispatch::<
            crate::prop::DynProp,
            crate::expr::DynExpr,
            crate::expr::DynExpr,
        >::Unreachable
    }
}

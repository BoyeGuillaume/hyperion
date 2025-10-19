use either::Either;
use smallvec::SmallVec;
use typed_arena::Arena;

use crate::{
    encoding::{EncodableExpr, tree::TreeBufNodeRef},
    expr::{AnyExprRef, Expr, variant::ExprType, view::ExprView},
};

pub trait ArenaAllocableExpr<'a> {
    fn alloc_in(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a ArenaAnyExpr<'a>;

    fn alloc_in_mut(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a mut ArenaAnyExpr<'a>;
}

pub struct ExprArenaCtx<'a> {
    arena: Arena<ArenaAnyExpr<'a>>,
}

impl<'a> ExprArenaCtx<'a> {
    pub fn new() -> Self {
        Self {
            arena: Arena::new(),
        }
    }

    pub fn alloc_expr(&'a self, expr: ArenaAnyExpr<'a>) -> &'a mut ArenaAnyExpr<'a> {
        self.arena.alloc(expr)
    }
}

#[derive(Clone)]
pub enum ArenaAnyExpr<'a> {
    ArenaView(ExprView<&'a ArenaAnyExpr<'a>, &'a ArenaAnyExpr<'a>, &'a ArenaAnyExpr<'a>>),
    ExprRef(AnyExprRef<'a>),
}

impl EncodableExpr for ArenaAnyExpr<'_> {
    fn encode_tree_step(&self, treebuf: &mut crate::encoding::tree::TreeBuf) -> TreeBufNodeRef {
        use ExprType::*;

        enum Frame<'a> {
            Enter(&'a ArenaAnyExpr<'a>),
            Exit(&'a ArenaAnyExpr<'a>),
        }

        let mut stack: SmallVec<[Frame<'_>; 16]> = SmallVec::new();
        let mut results: SmallVec<[TreeBufNodeRef; 16]> = SmallVec::new();

        stack.push(Frame::Enter(self));

        while let Some(frame) = stack.pop() {
            match frame {
                Frame::Enter(node) => match node {
                    ArenaAnyExpr::ExprRef(r) => {
                        // Treat borrowed encoded subtrees as leaves by copying them in
                        results.push(r.encode_tree_step(treebuf));
                    }
                    ArenaAnyExpr::ArenaView(view) => {
                        // Post-order: visit children, then build this node
                        stack.push(Frame::Exit(node));
                        match view {
                            ExprView::Variable(_)
                            | ExprView::Bool
                            | ExprView::Omega
                            | ExprView::True
                            | ExprView::False
                            | ExprView::Never => {}
                            ExprView::Not(e) | ExprView::Powerset(e) => {
                                stack.push(Frame::Enter(e));
                            }
                            ExprView::And(a, b)
                            | ExprView::Or(a, b)
                            | ExprView::Implies(a, b)
                            | ExprView::Iff(a, b)
                            | ExprView::Equal(a, b)
                            | ExprView::Tuple(a, b) => {
                                stack.push(Frame::Enter(b));
                                stack.push(Frame::Enter(a));
                            }
                            ExprView::Lambda { arg, body }
                            | ExprView::Call {
                                func: arg,
                                arg: body,
                            } => {
                                // Maintain field semantics explicitly
                                stack.push(Frame::Enter(body));
                                stack.push(Frame::Enter(arg));
                            }
                            ExprView::Forall { dtype, inner, .. }
                            | ExprView::Exists { dtype, inner, .. } => {
                                stack.push(Frame::Enter(inner));
                                stack.push(Frame::Enter(dtype));
                            }
                            ExprView::If {
                                condition,
                                then_branch,
                                else_branch,
                            } => {
                                stack.push(Frame::Enter(else_branch));
                                stack.push(Frame::Enter(then_branch));
                                stack.push(Frame::Enter(condition));
                            }
                        }
                    }
                },
                Frame::Exit(node) => match node {
                    ArenaAnyExpr::ExprRef(_) => {
                        // Already handled as leaf
                    }
                    ArenaAnyExpr::ArenaView(view) => {
                        let noderef = match view {
                            ExprView::Variable(v) => {
                                treebuf.push_node(Variable as u8, Some(v.raw()), &[])
                            }
                            ExprView::Bool => treebuf.push_node(Bool as u8, None, &[]),
                            ExprView::Omega => treebuf.push_node(Omega as u8, None, &[]),
                            ExprView::True => treebuf.push_node(True as u8, None, &[]),
                            ExprView::False => treebuf.push_node(False as u8, None, &[]),
                            ExprView::Never => treebuf.push_node(Never as u8, None, &[]),
                            ExprView::Not(_) => {
                                let c = results.pop().unwrap();
                                treebuf.push_node(Not as u8, None, &[c])
                            }
                            ExprView::Powerset(_) => {
                                let c = results.pop().unwrap();
                                treebuf.push_node(Powerset as u8, None, &[c])
                            }
                            ExprView::And(_, _) => {
                                let r = results.pop().unwrap();
                                let l = results.pop().unwrap();
                                treebuf.push_node(And as u8, None, &[l, r])
                            }
                            ExprView::Or(_, _) => {
                                let r = results.pop().unwrap();
                                let l = results.pop().unwrap();
                                treebuf.push_node(Or as u8, None, &[l, r])
                            }
                            ExprView::Implies(_, _) => {
                                let r = results.pop().unwrap();
                                let l = results.pop().unwrap();
                                treebuf.push_node(Implies as u8, None, &[l, r])
                            }
                            ExprView::Iff(_, _) => {
                                let r = results.pop().unwrap();
                                let l = results.pop().unwrap();
                                treebuf.push_node(Iff as u8, None, &[l, r])
                            }
                            ExprView::Equal(_, _) => {
                                let r = results.pop().unwrap();
                                let l = results.pop().unwrap();
                                treebuf.push_node(Equal as u8, None, &[l, r])
                            }
                            ExprView::Lambda { .. } => {
                                let body = results.pop().unwrap();
                                let arg = results.pop().unwrap();
                                treebuf.push_node(Lambda as u8, None, &[arg, body])
                            }
                            ExprView::Call { .. } => {
                                let arg = results.pop().unwrap();
                                let func = results.pop().unwrap();
                                treebuf.push_node(Call as u8, None, &[func, arg])
                            }
                            ExprView::Tuple(_, _) => {
                                let r = results.pop().unwrap();
                                let l = results.pop().unwrap();
                                treebuf.push_node(Tuple as u8, None, &[l, r])
                            }
                            ExprView::Forall { variable, .. } => {
                                let inner = results.pop().unwrap();
                                let dtype = results.pop().unwrap();
                                treebuf.push_node(
                                    Forall as u8,
                                    Some(variable.raw()),
                                    &[dtype, inner],
                                )
                            }
                            ExprView::Exists { variable, .. } => {
                                let inner = results.pop().unwrap();
                                let dtype = results.pop().unwrap();
                                treebuf.push_node(
                                    Exists as u8,
                                    Some(variable.raw()),
                                    &[dtype, inner],
                                )
                            }
                            ExprView::If { .. } => {
                                let else_b = results.pop().unwrap();
                                let then_b = results.pop().unwrap();
                                let cond = results.pop().unwrap();
                                treebuf.push_node(If as u8, None, &[cond, then_b, else_b])
                            }
                        };
                        results.push(noderef);
                    }
                },
            }
        }

        debug_assert_eq!(results.len(), 1);
        results.pop().unwrap()
    }
}

impl<'a> Expr for ArenaAnyExpr<'a> {
    fn view(&self) -> ExprView<impl Expr, impl Expr, impl Expr> {
        match self {
            ArenaAnyExpr::ArenaView(view) => view.clone().map_unary(|x, _| Either::Left(x)),
            ArenaAnyExpr::ExprRef(any_expr_ref) => {
                any_expr_ref.view_typed().map_unary(|x, _| Either::Right(x))
            }
        }
    }
}

impl<'a> ArenaAllocableExpr<'a> for &'a ArenaAnyExpr<'a> {
    fn alloc_in(&self, _ctx: &'a ExprArenaCtx<'a>) -> &'a ArenaAnyExpr<'a> {
        self
    }

    fn alloc_in_mut(&self, ctx: &'a ExprArenaCtx<'a>) -> &'a mut ArenaAnyExpr<'a> {
        ctx.alloc_expr((*self).clone())
    }
}

pub fn with_arena_ctx<F>(callback: F)
where
    F: for<'a> FnOnce(ExprArenaCtx<'a>),
{
    let ctx = ExprArenaCtx {
        arena: Arena::new(),
    };
    callback(ctx);
}

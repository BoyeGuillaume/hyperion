//! RcDoc-based pretty-printer with termcolor annotations for `Expr`.
//!
//! This module builds annotated `RcDoc<Style>` trees from `Expr` and renders
//! them to a `termcolor::WriteColor` sink with width-aware layout.

use crate::expr::view::ExprDispatchVariant;
use crate::expr::{Expr, view::ExprView};
use crate::variable::InlineVariable;
use pretty::{RcDoc, RenderAnnotated};
use std::io::{self, Write};
use strum::IntoDiscriminant;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

/// Styles that we annotate parts of the document with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    Punct, // commas, arrows, colons, periods
    /// Parentheses are colored by nesting depth so matching pairs share a color.
    Paren(u8),
    Keyword,  // if, then, else, forall, exists
    Operator, // +, *, &&, ||, =>, <=>, ==
    Ident,    // variables
    Type,     // type constructors like Bool, Omega, Never
}

impl Style {
    fn to_color_spec(self) -> ColorSpec {
        let mut s = ColorSpec::new();
        match self {
            Style::Punct => {
                s.set_dimmed(true);
            }
            Style::Paren(depth) => {
                // Rotate through a palette for nested parentheses. Use intense colors for clarity.
                let fg = match depth % 6 {
                    0 => Color::Blue,
                    1 => Color::Green,
                    2 => Color::White,
                    3 => Color::Yellow,
                    4 => Color::Red,
                    5 => Color::Magenta,
                    _ => unreachable!(),
                };
                s.set_fg(Some(fg)).set_dimmed(true);
            }
            Style::Keyword => {
                s.set_fg(Some(Color::Cyan)).set_bold(true);
            }
            Style::Operator => {
                s.set_fg(Some(Color::Yellow)).set_bold(true);
            }
            Style::Ident => {
                s.set_fg(Some(Color::Green)).set_bold(true);
            }
            Style::Type => {
                s.set_fg(Some(Color::Magenta));
            }
        }
        s
    }
}

fn styled(style: Style, s: &'static str) -> RcDoc<'static, Style> {
    RcDoc::as_string(s).annotate(style)
}

fn punct(s: &'static str) -> RcDoc<'static, Style> {
    styled(Style::Punct, s)
}

#[inline]
fn lparen(depth: u8) -> RcDoc<'static, Style> {
    RcDoc::as_string("(").annotate(Style::Paren(depth))
}

#[inline]
fn rparen(depth: u8) -> RcDoc<'static, Style> {
    RcDoc::as_string(")").annotate(Style::Paren(depth))
}

fn kw(s: &'static str) -> RcDoc<'static, Style> {
    styled(Style::Keyword, s)
}

fn op(s: &'static str) -> RcDoc<'static, Style> {
    styled(Style::Operator, s)
}

fn ident(v: InlineVariable) -> RcDoc<'static, Style> {
    RcDoc::as_string(v).annotate(Style::Ident)
}

pub fn calculate_precedence(e: ExprDispatchVariant) -> u8 {
    use ExprDispatchVariant::*;

    match e {
        App | If | ForAll | Exists => 2,
        Func | Tuple => 1,
        And | Or | Implies | Iff | Equal => 2,
        Not => 3,
        Var | Never | True | False | Bool | Omega | Powerset => 4,
        // _ => unreachable!(),
    }
}

#[inline]
fn requires_parens(
    current_type: ExprDispatchVariant,
    parent_type: Option<ExprDispatchVariant>,
) -> bool {
    match parent_type {
        None => false,
        Some(pt) => {
            let current_prec = calculate_precedence(current_type);
            let parent_prec = calculate_precedence(pt);
            (parent_prec > current_prec) || (parent_prec == current_prec && current_type != pt)
        }
    }
}

#[inline]
pub fn to_doc_parenthesized_with_depth<E: Expr>(
    e: &E,
    parent_type: ExprDispatchVariant,
    depth: u8,
) -> RcDoc<'static, Style> {
    let current_type = e.view_expr().discriminant();
    let need = requires_parens(current_type, Some(parent_type));
    if need {
        lparen(depth)
            .append(to_doc_with_depth(e, depth + 1))
            .append(rparen(depth))
            .group()
    } else {
        to_doc_with_depth(e, depth)
    }
}

/// Depth-aware variant that colors parentheses by nesting level.
pub fn to_doc_with_depth<E: Expr>(e: &E, depth: u8) -> RcDoc<'static, Style> {
    match e.view_expr() {
        // Term-level
        ExprView::Var(v) => ident(v),
        ExprView::Never => op("<>"), // or "unreachable"
        ExprView::App { func, arg } => ident(func)
            .append(lparen(depth))
            .append(to_doc_with_depth(&arg, depth + 1))
            .append(rparen(depth))
            .group(),
        ExprView::If {
            condition,
            then_branch,
            else_branch,
        } => kw("if")
            .append(RcDoc::space())
            .append(to_doc_parenthesized_with_depth(
                &condition,
                ExprDispatchVariant::If,
                depth,
            ))
            .append(RcDoc::line())
            .append(kw("then"))
            .append(RcDoc::space())
            .append(to_doc_parenthesized_with_depth(
                &then_branch,
                ExprDispatchVariant::If,
                depth,
            ))
            .append(RcDoc::line())
            .append(kw("else"))
            .append(RcDoc::space())
            .append(to_doc_parenthesized_with_depth(
                &else_branch,
                ExprDispatchVariant::If,
                depth,
            ))
            .group()
            .nest(2),
        ExprView::Tuple(a, b) => {
            to_doc_parenthesized_with_depth(&a, ExprDispatchVariant::Tuple, depth + 1)
                .append(punct(", "))
                .append(to_doc_parenthesized_with_depth(
                    &b,
                    ExprDispatchVariant::Tuple,
                    depth + 1,
                ))
                .group()
        }

        // Logic-level
        ExprView::True => kw("true"),
        ExprView::False => kw("false"),
        ExprView::Not(p) => op("!")
            .append(to_doc_parenthesized_with_depth(
                &p,
                ExprDispatchVariant::Not,
                depth,
            ))
            .group(),
        ExprView::And(a, b) => to_doc_parenthesized_with_depth(&a, ExprDispatchVariant::And, depth)
            .append(RcDoc::space())
            .append(op("/\\"))
            .append(RcDoc::space())
            .append(to_doc_parenthesized_with_depth(
                &b,
                ExprDispatchVariant::And,
                depth,
            ))
            .group(),
        ExprView::Or(a, b) => to_doc_parenthesized_with_depth(&a, ExprDispatchVariant::Or, depth)
            .append(RcDoc::space())
            .append(op("\\/"))
            .append(RcDoc::space())
            .append(to_doc_parenthesized_with_depth(
                &b,
                ExprDispatchVariant::Or,
                depth,
            ))
            .group(),
        ExprView::Implies(a, b) => {
            to_doc_parenthesized_with_depth(&a, ExprDispatchVariant::Implies, depth)
                .append(RcDoc::space())
                .append(op("=>"))
                .append(RcDoc::space())
                .append(to_doc_parenthesized_with_depth(
                    &b,
                    ExprDispatchVariant::Implies,
                    depth,
                ))
                .group()
        }
        ExprView::Iff(a, b) => to_doc_parenthesized_with_depth(&a, ExprDispatchVariant::Iff, depth)
            .append(RcDoc::space())
            .append(op("<=>"))
            .append(RcDoc::space())
            .append(to_doc_parenthesized_with_depth(
                &b,
                ExprDispatchVariant::Iff,
                depth,
            ))
            .group(),
        ExprView::ForAll {
            variable,
            dtype,
            inner,
        } => kw("forall")
            .append(RcDoc::space())
            .append(ident(variable))
            .append(RcDoc::space())
            .append(punct(":"))
            .append(RcDoc::space())
            .append(to_doc_parenthesized_with_depth(
                &dtype,
                ExprDispatchVariant::ForAll,
                depth,
            ))
            .append(RcDoc::space())
            .append(punct("."))
            .append(RcDoc::space())
            .append(to_doc_parenthesized_with_depth(
                &inner,
                ExprDispatchVariant::ForAll,
                depth,
            ))
            .group(),
        ExprView::Exists {
            variable,
            dtype,
            inner,
        } => kw("exists")
            .append(RcDoc::space())
            .append(ident(variable))
            .append(RcDoc::space())
            .append(punct(":"))
            .append(RcDoc::space())
            .append(to_doc_parenthesized_with_depth(
                &dtype,
                ExprDispatchVariant::ForAll,
                depth,
            ))
            .append(RcDoc::space())
            .append(punct("."))
            .append(RcDoc::space())
            .append(to_doc_parenthesized_with_depth(
                &inner,
                ExprDispatchVariant::ForAll,
                depth,
            ))
            .group(),
        ExprView::Equal(a, b) => {
            to_doc_parenthesized_with_depth(&a, ExprDispatchVariant::Equal, depth)
                .append(RcDoc::space())
                .append(op("="))
                .append(RcDoc::space())
                .append(to_doc_parenthesized_with_depth(
                    &b,
                    ExprDispatchVariant::Equal,
                    depth,
                ))
                .group()
        }

        // Type-level
        ExprView::Bool => styled(Style::Type, "Bool"),
        ExprView::Omega => styled(Style::Type, "Omega"),
        ExprView::Powerset(a) => styled(Style::Type, "Powerset")
            .append(lparen(depth))
            .append(to_doc_with_depth(&a, depth + 1))
            .append(rparen(depth))
            .group(),
        ExprView::Func(a, b) => {
            to_doc_parenthesized_with_depth(&a, ExprDispatchVariant::Func, depth)
                .append(RcDoc::space())
                .append(punct("->"))
                .append(RcDoc::space())
                .append(to_doc_parenthesized_with_depth(
                    &b,
                    ExprDispatchVariant::Func,
                    depth,
                ))
                .group()
        }
    }
}

// A writer that maps Style annotations to termcolor ColorSpec on a WriteColor sink.
struct ColorWriter<'w, W: WriteColor + Write> {
    out: &'w mut W,
}

impl<'a, 'w, W: WriteColor + Write> RenderAnnotated<'a, Style> for ColorWriter<'w, W> {
    fn push_annotation(&mut self, ann: &'a Style) -> io::Result<()> {
        self.out.set_color(&ann.to_color_spec())
    }
    fn pop_annotation(&mut self) -> io::Result<()> {
        self.out.reset()
    }
}

impl<'w, W: WriteColor + Write> pretty::Render for ColorWriter<'w, W> {
    type Error = io::Error;
    fn write_str(&mut self, s: &str) -> io::Result<usize> {
        self.out.write_all(s.as_bytes())?;
        Ok(s.len())
    }
    fn write_str_all(&mut self, s: &str) -> io::Result<()> {
        self.out.write_all(s.as_bytes())
    }
    fn fail_doc(&self) -> Self::Error {
        io::Error::new(io::ErrorKind::Other, "render failed")
    }
}

/// Render a document to a `termcolor::WriteColor` with width-aware layout.
pub fn render_to<W: WriteColor + Write>(
    doc: &RcDoc<'_, Style>,
    width: usize,
    out: &mut W,
) -> std::io::Result<()> {
    let mut cw = ColorWriter { out };
    doc.render_raw(width, &mut cw)
}

/// Convenience: print to stdout with colors if supported.
pub fn print_colored<E: Expr>(e: &E, width: usize) -> std::io::Result<()> {
    let stdout = StandardStream::stdout(ColorChoice::Auto);
    let mut stdout = stdout.lock();
    let doc = to_doc_with_depth(e, 0);
    render_to(&doc, width, &mut stdout)
}

/// Convenience: format to a plain string without colors.
pub fn to_plain_string<E: Expr>(e: &E, width: usize) -> String {
    let mut buf = String::new();
    let _ = to_doc_with_depth(e, 0).render_fmt(width, &mut buf);
    buf
}

/// Convenience: retrieve the width of the terminal, or 80 if it cannot be determined.
pub fn terminal_width() -> usize {
    term_size::dimensions().map(|(w, _)| w).unwrap_or(80)
}

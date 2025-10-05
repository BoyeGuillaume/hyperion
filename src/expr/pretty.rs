//! RcDoc-based pretty-printer with termcolor annotations for `Expr`.
//!
//! This module builds annotated `RcDoc<Style>` trees from `Expr` and renders
//! them to a `termcolor::WriteColor` sink with width-aware layout.

use crate::expr::{Expr, view::ExprView};
use crate::variable::InlineVariable;
use pretty::{RcDoc, RenderAnnotated};
use std::io::{self, Write};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

/// Styles that we annotate parts of the document with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    Punct,    // parentheses, commas, arrows
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

fn kw(s: &'static str) -> RcDoc<'static, Style> {
    styled(Style::Keyword, s)
}

fn op(s: &'static str) -> RcDoc<'static, Style> {
    styled(Style::Operator, s)
}

fn ident(v: InlineVariable) -> RcDoc<'static, Style> {
    RcDoc::as_string(v).annotate(Style::Ident)
}

/// Build a pretty document for any `Expr` by pattern-matching on its `ExprView`.
pub fn to_doc<E: Expr>(e: &E) -> RcDoc<'static, Style> {
    match e.view_expr() {
        // Term-level
        ExprView::Var(v) => ident(v),
        ExprView::Unreachable => kw("unreachable"),
        ExprView::App { func, arg } => ident(func)
            .append(punct("("))
            .append(to_doc(&arg))
            .append(punct(")"))
            .group(),
        ExprView::If {
            condition,
            then_branch,
            else_branch,
        } => kw("if")
            .append(RcDoc::space())
            .append(to_doc(&condition))
            .append(RcDoc::line())
            .append(kw("then"))
            .append(RcDoc::space())
            .append(to_doc(&then_branch))
            .append(RcDoc::line())
            .append(kw("else"))
            .append(RcDoc::space())
            .append(to_doc(&else_branch))
            .group()
            .nest(2),
        ExprView::Tuple(a, b) => punct("(")
            .append(to_doc(&a))
            .append(punct(", "))
            .append(to_doc(&b))
            .append(punct(")"))
            .group(),

        // Logic-level
        ExprView::True => kw("true"),
        ExprView::False => kw("false"),
        ExprView::Not(p) => op("¬").append(to_doc(&p)).group(),
        ExprView::And(a, b) => to_doc(&a)
            .append(RcDoc::space())
            .append(op("∧"))
            .append(RcDoc::space())
            .append(to_doc(&b))
            .group(),
        ExprView::Or(a, b) => to_doc(&a)
            .append(RcDoc::space())
            .append(op("∨"))
            .append(RcDoc::space())
            .append(to_doc(&b))
            .group(),
        ExprView::Implies(a, b) => to_doc(&a)
            .append(RcDoc::space())
            .append(op("=>"))
            .append(RcDoc::space())
            .append(to_doc(&b))
            .group(),
        ExprView::Iff(a, b) => to_doc(&a)
            .append(RcDoc::space())
            .append(op("<=>"))
            .append(RcDoc::space())
            .append(to_doc(&b))
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
            .append(to_doc(&dtype))
            .append(RcDoc::space())
            .append(punct("."))
            .append(RcDoc::space())
            .append(to_doc(&inner))
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
            .append(to_doc(&dtype))
            .append(RcDoc::space())
            .append(punct("."))
            .append(RcDoc::space())
            .append(to_doc(&inner))
            .group(),
        ExprView::Equal(a, b) => to_doc(&a)
            .append(RcDoc::space())
            .append(op("="))
            .append(RcDoc::space())
            .append(to_doc(&b))
            .group(),

        // Type-level
        ExprView::Bool => styled(Style::Type, "Bool"),
        ExprView::Omega => styled(Style::Type, "Ω"),
        ExprView::Never => styled(Style::Type, "Never"),
        ExprView::Powerset(a) => styled(Style::Type, "P")
            .append(punct("("))
            .append(to_doc(&a))
            .append(punct(")"))
            .group(),
        ExprView::Func(a, b) => to_doc(&a)
            .append(RcDoc::space())
            .append(punct("->"))
            .append(RcDoc::space())
            .append(to_doc(&b))
            .group(),
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
    let doc = to_doc(e);
    render_to(&doc, width, &mut stdout)
}

/// Convenience: format to a plain string without colors.
pub fn to_plain_string<E: Expr>(e: &E, width: usize) -> String {
    let mut buf = String::new();
    let _ = to_doc(e).render_fmt(width, &mut buf);
    buf
}

/// Convenience: retrieve the width of the terminal, or 80 if it cannot be determined.
pub fn terminal_width() -> usize {
    term_size::dimensions().map(|(w, _)| w).unwrap_or(80)
}

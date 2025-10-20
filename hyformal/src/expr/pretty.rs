//! RcDoc-based pretty-printer with termcolor annotations for `Expr`.
//!
//! Role
//! - Convert an `Expr` into an annotated document suitable for width-aware rendering.
//! - Provide colored output for terminals (TTY-aware) and plain strings for logs/tests.
//!
//! Performance
//! - Building the doc is O(n) in expression size; rendering respects line widths with
//!   linear-time layout in the size of the resulting document.

use crate::expr::defs::*;
use crate::expr::{AnyExpr, AnyExprRef};
use crate::expr::{Expr, variant::ExprType, view::ExprView};
use crate::variable::InlineVariable;
use pretty::{FmtWrite, RcDoc, RenderAnnotated};
use std::io::{self, Write};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

/// Styles used to annotate parts of the pretty-printed document.
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

fn calculate_precedence(e: ExprType) -> u8 {
    use ExprType::*;

    match e {
        If => 1,
        Forall | Exists | Lambda => 2,
        And | Or | Implies | Iff => 3,
        Equal => 4,
        Not => 5,
        Tuple | Call => 6,
        Powerset => 7,
        Variable | Never | True | False | Bool | Omega => 255,
    }
}

#[inline]
fn requires_parens(current_type: ExprType, parent_type: Option<ExprType>) -> bool {
    match parent_type {
        None => false,
        Some(pt) => {
            let allow_self_parens = !matches!(
                current_type,
                ExprType::If | ExprType::Call | ExprType::Lambda | ExprType::Implies
            );

            let current_prec = calculate_precedence(current_type);
            let parent_prec = calculate_precedence(pt);
            (parent_prec > current_prec)
                || (parent_prec == current_prec && current_type != pt)
                || (parent_prec == current_prec && !allow_self_parens)
        }
    }
}

#[inline]
fn to_doc_parenthesized_with_depth<E: Expr>(
    e: &E,
    parent_type: ExprType,
    depth: u8,
) -> RcDoc<'static, Style> {
    let current_type = e.view().r#type();
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
fn to_doc_with_depth<E: Expr>(e: &E, depth: u8) -> RcDoc<'static, Style> {
    match e.view() {
        // Term-level
        ExprView::Variable(v) => ident(v),
        ExprView::Never => op("<>"), // or "unreachable"
        ExprView::Call { func, arg } => {
            to_doc_parenthesized_with_depth(&func, ExprType::Call, depth)
                .append(lparen(depth))
                .append(to_doc_with_depth(&arg, depth + 1))
                .append(rparen(depth))
                .group()
        }
        ExprView::If {
            condition,
            then_branch,
            else_branch,
        } => kw("if")
            .append(RcDoc::space())
            .append(to_doc_parenthesized_with_depth(
                &condition,
                ExprType::If,
                depth,
            ))
            .append(RcDoc::line())
            .append(kw("then"))
            .append(RcDoc::space())
            .append(to_doc_parenthesized_with_depth(
                &then_branch,
                ExprType::If,
                depth,
            ))
            .append(RcDoc::line())
            .append(kw("else"))
            .append(RcDoc::space())
            .append(to_doc_parenthesized_with_depth(
                &else_branch,
                ExprType::If,
                depth,
            ))
            .group()
            .nest(2),
        ExprView::Tuple(a, b) => to_doc_parenthesized_with_depth(&a, ExprType::Tuple, depth + 1)
            .append(punct(", "))
            .append(to_doc_parenthesized_with_depth(
                &b,
                ExprType::Tuple,
                depth + 1,
            ))
            .group(),

        // Lambda (binder-like)
        ExprView::Lambda { arg, body } => to_doc_with_depth(&arg, depth + 1)
            .append(RcDoc::space())
            .append(punct("->"))
            .append(RcDoc::space())
            .append(to_doc_parenthesized_with_depth(
                &body,
                ExprType::Lambda,
                depth,
            ))
            .group(),

        // Logic-level
        ExprView::True => kw("true"),
        ExprView::False => kw("false"),
        ExprView::Not(p) => op("!")
            .append(to_doc_parenthesized_with_depth(&p, ExprType::Not, depth))
            .group(),
        ExprView::And(a, b) => to_doc_parenthesized_with_depth(&a, ExprType::And, depth)
            .append(RcDoc::space())
            .append(op("/\\"))
            .append(RcDoc::space())
            .append(to_doc_parenthesized_with_depth(&b, ExprType::And, depth))
            .group(),
        ExprView::Or(a, b) => to_doc_parenthesized_with_depth(&a, ExprType::Or, depth)
            .append(RcDoc::space())
            .append(op("\\/"))
            .append(RcDoc::space())
            .append(to_doc_parenthesized_with_depth(&b, ExprType::Or, depth))
            .group(),
        ExprView::Implies(a, b) => to_doc_parenthesized_with_depth(&a, ExprType::Implies, depth)
            .append(RcDoc::space())
            .append(op("=>"))
            .append(RcDoc::space())
            .append(to_doc_parenthesized_with_depth(
                &b,
                ExprType::Implies,
                depth,
            ))
            .group(),
        ExprView::Iff(a, b) => to_doc_parenthesized_with_depth(&a, ExprType::Iff, depth)
            .append(RcDoc::space())
            .append(op("<=>"))
            .append(RcDoc::space())
            .append(to_doc_parenthesized_with_depth(&b, ExprType::Iff, depth))
            .group(),
        ExprView::Forall {
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
                ExprType::Forall,
                depth,
            ))
            .append(RcDoc::space())
            .append(punct("."))
            .append(RcDoc::space())
            .append(to_doc_parenthesized_with_depth(
                &inner,
                ExprType::Forall,
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
                ExprType::Exists,
                depth,
            ))
            .append(RcDoc::space())
            .append(punct("."))
            .append(RcDoc::space())
            .append(to_doc_parenthesized_with_depth(
                &inner,
                ExprType::Exists,
                depth,
            ))
            .group(),
        ExprView::Equal(a, b) => to_doc_parenthesized_with_depth(&a, ExprType::Equal, depth)
            .append(RcDoc::space())
            .append(op("="))
            .append(RcDoc::space())
            .append(to_doc_parenthesized_with_depth(&b, ExprType::Equal, depth))
            .group(),

        // Type-level
        ExprView::Bool => styled(Style::Type, "Bool"),
        ExprView::Omega => styled(Style::Type, "Omega"),
        ExprView::Powerset(a) => styled(Style::Type, "Powerset")
            .append(lparen(depth))
            .append(to_doc_with_depth(&a, depth + 1))
            .append(rparen(depth))
            .group(),
        // No separate Func node in the new AST; function-like constructs can be printed
        // via Lambda above.
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
        io::Error::other("render failed")
    }
}

/// Render a document to a `termcolor::WriteColor` with width-aware layout.
fn render_to<W: WriteColor + Write>(
    doc: &RcDoc<'_, Style>,
    width: usize,
    out: &mut W,
) -> std::io::Result<()> {
    let mut cw = ColorWriter { out };
    doc.render_raw(width, &mut cw)
}

/// Convenience: print to stdout with colors if supported.
fn print_colored<E: Expr>(e: &E, width: usize) -> std::io::Result<()> {
    let stdout = StandardStream::stdout(ColorChoice::Auto);
    let mut stdout = stdout.lock();
    let doc = to_doc_with_depth(e, 0);
    render_to(&doc, width, &mut stdout)
}

/// Convenience: format to a plain string without colors.
fn to_plain_string<E: Expr>(e: &E, width: usize) -> String {
    let mut buf = String::new();
    let _ = to_doc_with_depth(e, 0).render_fmt(width, &mut buf);
    buf
}

/// Convenience: retrieve the width of the terminal, or 80 if it cannot be determined.
fn terminal_width() -> usize {
    term_size::dimensions().map(|(w, _)| w).unwrap_or(80)
}

/// ======================== Trait impls =========================
/// Pretty-printing conveniences for any `Expr`.
pub trait PrettyExpr {
    /// Build an RcDoc representation of this expression with style annotations.
    /// Useful for composing or rendering manually.
    fn pretty_doc(&self) -> RcDoc<'static, Style>;

    /// Render this expression with colors to any termcolor writer at the given width.
    fn pretty_render_to<W: WriteColor + Write>(&self, width: usize, out: &mut W) -> io::Result<()>;

    /// Print this expression to stdout with colors (TTY-aware), at auto-detected width (or 80 if not a TTY).
    fn pretty_print(&self) -> io::Result<()>;

    /// Format this expression into a plain string (no colors)
    fn pretty_string(&self) -> String;
}

impl<T: Expr> PrettyExpr for T {
    #[inline]
    fn pretty_doc(&self) -> RcDoc<'static, Style> {
        to_doc_with_depth(self, 0)
    }

    #[inline]
    fn pretty_render_to<W: WriteColor + Write>(&self, width: usize, out: &mut W) -> io::Result<()> {
        let doc = self.pretty_doc();
        render_to(&doc, width, out)
    }

    #[inline]
    fn pretty_print(&self) -> io::Result<()> {
        let width = terminal_width();
        print_colored(self, width)
    }

    #[inline]
    fn pretty_string(&self) -> String {
        to_plain_string(self, 80)
    }
}

macro_rules! impl_display_for_type {
    (
        $t:ident $(
            < $($gen:tt),* >
        )?
    ) => {
        impl $(
            < $($gen: Expr),* >
        )? std::fmt::Display for $t $(< $($gen),* >)? {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let mut w = FmtWrite::new(f);
                let doc = self.pretty_doc();
                doc.render_raw(80, &mut w)
            }
        }
    };
}

impl_display_for_type!(Bool);
impl_display_for_type!(Omega);
impl_display_for_type!(True);
impl_display_for_type!(False);
impl_display_for_type!(Never);

impl_display_for_type!(Not<A>);
impl_display_for_type!(Powerset<A>);

impl_display_for_type!(And<A, B>);
impl_display_for_type!(Or<A, B>);
impl_display_for_type!(Implies<A, B>);
impl_display_for_type!(Iff<A, B>);
impl_display_for_type!(Equal<A, B>);
impl_display_for_type!(Lambda<A, B>);
impl_display_for_type!(Call<A, B>);
impl_display_for_type!(Tuple<A, B>);
impl_display_for_type!(ForAll<A, B>);
impl_display_for_type!(Exists<A, B>);

impl_display_for_type!(If<A, B, C>);

impl_display_for_type!(AnyExpr);
impl<'a> std::fmt::Display for AnyExprRef<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut w = FmtWrite::new(f);
        let doc = self.pretty_doc();
        doc.render_raw(80, &mut w)
    }
}

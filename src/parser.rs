//! Parser for the pretty-printed unified expression language using chumsky.
//!
//! Role
//! - Turn human-readable expressions into compact encoded [`AnyExpr`](crate::expr::AnyExpr).
//! - Mirrors the precedence and associativity used by the pretty-printer for round-tripping.
//!
//! Two stages:
//! 1) Tokenisation from input string to a `Token` stream.
//! 2) Parsing tokens into a lightweight arena-allocated AST, then encoding to `AnyExpr`.
//!
//! The accepted syntax round-trips with the pretty-printer in `expr::pretty`:
//! - Variables: single letter a..z or A..Z map to ids 0..25. Larger ids: `v<number>`
//!   (primary) or `_number` (also accepted) map to raw id `26 + number`.
//! - Literals/types: true, false, Bool, Omega, `<>` (Never), `Powerset(expr)`.
//! - Application: `f(expr)` where `f` is any expression (not only variables).
//! - Tuples: `A, B` (comma is the lowest-precedence binary operator; left-associative).
//! - Lambda: `A -> B` (right-associative, lower precedence than logic operators).
//! - Conditionals: `if P then X else Y`.
//! - Logic: `!P`, `P /\ Q`, `P \/ Q`, `P => Q`, `P <=> Q`, `A = B`.
//! - Quantifiers: `forall x : T . P` and `exists x : T . P`.
//!
//! Note: Parentheses can wrap any full expression; we follow a precedence hierarchy
//! compatible with the pretty-printer (If < Forall/Exists/Lambda < And/Or/Iff/Implies <
//! Equal < Not < Tuple/Call < Powerset < atoms).
use std::cell::RefCell;

use chumsky::{input::ValueInput, prelude::*};

use crate::arena::{ArenaAnyExpr, ExprArenaCtx};
use crate::expr::{AnyExpr, Expr};
use crate::variable::InlineVariable;
// Note: We keep the lexer unchanged. The parsing portion below now uses chumsky combinators
// directly over the token stream, replacing the previous TS/AST hand-rolled parser.

pub type Spanned<T> = (T, SimpleSpan);
type Span = SimpleSpan;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Token {
    // Delimiters & punctuation
    LParen,
    RParen,
    Comma,
    Colon,
    Dot,

    // Operators
    Not,
    And,      // /\
    Or,       // \/
    Implies,  // =>
    Iff,      // <=>
    Equal,    // =
    Arrow,    // -> (lambda)
    NeverSym, // <>

    // Keywords
    If,
    Then,
    Else,
    ForAll,
    Exists,
    True,
    False,
    Bool,
    Omega,
    Powerset,

    // Identifiers
    Var(InlineVariable), // directly parsed variable
}

// ---------------- Lexer ----------------
fn raw_variable_lexer<'a>() -> impl Parser<'a, &'a str, Token, extra::Err<Rich<'a, char>>> {
    // Single-letter variables a..z or A..Z
    one_of("$%")
        .then(
            any()
                .filter(|c: &char| c.is_ascii_hexdigit())
                .repeated(),
        )
        .to_slice()
        .try_map(|s: &str, span| -> Result<Token, Rich<char>> {
            let var = InlineVariable::from_string(s).map_err(|msg| {
                Rich::custom(span, format!(
                    "invalid variable '{s}': expected format '$<hex>' or '%<hex>' (e.g., $1a2b3c). {msg}"
                ))
            })?;
            Ok(Token::Var(var))
        })
}

fn lexer<'a>() -> impl Parser<'a, &'a str, Vec<Spanned<Token>>, extra::Err<Rich<'a, char>>> {
    // Multi-char operators/keywords first to avoid prefix capture
    let iff = just("<=>").to(Token::Iff);
    let implies = just("=>").to(Token::Implies);
    let arrow = just("->").to(Token::Arrow);
    let never = just("<>").to(Token::NeverSym);
    let and_op = just("/\\").to(Token::And);
    let or_op = just("\\/").to(Token::Or);

    // Keywords
    // Use word boundary: ensure next char isn't an ASCII alphanumeric or underscore
    let keywords = choice((
        just("if").map(|_| Token::If),
        just("then").map(|_| Token::Then),
        just("else").map(|_| Token::Else),
        just("forall").map(|_| Token::ForAll),
        just("exists").map(|_| Token::Exists),
        just("true").map(|_| Token::True),
        just("false").map(|_| Token::False),
        just("Bool").map(|_| Token::Bool),
        just("Omega").map(|_| Token::Omega),
        just("Powerset").map(|_| Token::Powerset),
    ));
    // let word = any()
    //     .filter(|c: &char| c.is_ascii_alphabetic())
    //     .then(
    //         any()
    //             .filter(|c: &char| c.is_ascii_alphanumeric())
    //             .repeated(),
    //     )
    //     .to_slice()
    //     .try_map(|s: &str, span| -> Result<Token, Rich<char>> {
    //         // Keywords
    //         let tok = match s {
    //             "if" => Token::If,
    //             "then" => Token::Then,
    //             "else" => Token::Else,
    //             "forall" => Token::ForAll,
    //             "exists" => Token::Exists,
    //             "true" => Token::True,
    //             "false" => Token::False,
    //             "Bool" => Token::Bool,
    //             "Omega" => Token::Omega,
    //             "Powerset" => Token::Powerset,
    //             _ => {
    //                 return Err(Rich::custom(
    //                     span,
    //                     format!(
    //                         "unrecognized identifier '{s}': expected a keyword like 'if', 'forall', 'Bool', etc."
    //                     ),
    //                 ));
    //             }
    //         };
    //         Ok(tok)
    //     });

    let punct = choice((
        just('(').to(Token::LParen),
        just(')').to(Token::RParen),
        just(',').to(Token::Comma),
        just(':').to(Token::Colon),
        just('.').to(Token::Dot),
        just('!').to(Token::Not),
        just('=').to(Token::Equal),
    ));

    let token = choice((
        // Operators (longest first)
        iff,
        implies,
        arrow,
        never,
        and_op,
        or_op,    // Keywords and identifiers
        keywords, // Punct/single char ops
        raw_variable_lexer(),
        punct,
    ));

    // Comments are single-line starting with ';'
    let comment = just(';')
        .then(any().and_is(just("\n").not()).repeated())
        .padded()
        .to(());

    token
        .map_with(|tok, e| (tok, e.span()))
        .padded_by(comment.repeated())
        .padded()
        .repeated()
        .collect()
        .then_ignore(end())
}

// ---------------- chumsky parser over tokens (arena-backed) ----------------

fn arena_expr_parser<'a, I>(
    ctx: &'a ExprArenaCtx<'a>,
) -> impl Parser<'a, I, &'a RefCell<ArenaAnyExpr<'a>>, extra::Err<Rich<'a, Token, Span>>> + Clone + 'a
where
    I: ValueInput<'a, Token = Token, Span = Span>,
{
    recursive(|expr| {
        // Identifiers/values
        let ident = select! { Token::Var(v) => v };

        let value = select! {
            Token::True => ctx.alloc_expr(ArenaAnyExpr::ArenaView(crate::expr::view::ExprView::True)) as &_,
            Token::False => ctx.alloc_expr(ArenaAnyExpr::ArenaView(crate::expr::view::ExprView::False)) as &_,
            Token::Bool => ctx.alloc_expr(ArenaAnyExpr::ArenaView(crate::expr::view::ExprView::Bool)) as &_,
            Token::Omega => ctx.alloc_expr(ArenaAnyExpr::ArenaView(crate::expr::view::ExprView::Omega)) as &_,
            Token::NeverSym => ctx.alloc_expr(ArenaAnyExpr::ArenaView(crate::expr::view::ExprView::Never)) as &_,
        };

        // Parenthesised expressions (recover mismatched/missing parens)
        let paren_expr = expr
            .clone()
            .delimited_by(just(Token::LParen), just(Token::RParen))
            .recover_with(via_parser(nested_delimiters(
                Token::LParen,
                Token::RParen,
                [],
                |_| {
                    ctx.alloc_expr(ArenaAnyExpr::ArenaView(crate::expr::view::ExprView::Never))
                        as &_
                },
            )))
            .labelled("parentheses");

        // Powerset(expr)
        let powerset = just(Token::Powerset)
            .ignore_then(
                expr.clone()
                    .delimited_by(just(Token::LParen), just(Token::RParen))
                    .recover_with(via_parser(nested_delimiters(
                        Token::LParen,
                        Token::RParen,
                        [],
                        |_| {
                            ctx.alloc_expr(ArenaAnyExpr::ArenaView(
                                crate::expr::view::ExprView::Never,
                            )) as &_
                        },
                    )))
                    .labelled("powerset-args"),
            )
            .map(|e| {
                let view = crate::expr::view::ExprView::Powerset(e);
                ctx.alloc_expr(ArenaAnyExpr::ArenaView(view)) as &_
            })
            .labelled("Powerset");

        // Atomic expressions (no ambiguity)
        let atom = value
            .or(ident.map(|v| {
                ctx.alloc_expr(ArenaAnyExpr::ArenaView(
                    crate::expr::view::ExprView::Variable(v),
                )) as &_
            }))
            .or(powerset)
            .or(paren_expr)
            .labelled("atom");

        // Calls (left-assoc, very high precedence)
        let call = atom
            .clone()
            .foldl(
                expr.clone()
                    .delimited_by(just(Token::LParen), just(Token::RParen))
                    .recover_with(via_parser(nested_delimiters(
                        Token::LParen,
                        Token::RParen,
                        [],
                        |_| {
                            ctx.alloc_expr(ArenaAnyExpr::ArenaView(
                                crate::expr::view::ExprView::Never,
                            )) as &_
                        },
                    )))
                    .labelled("call-args")
                    .repeated(),
                |f, a| {
                    let view = crate::expr::view::ExprView::Call { func: f, arg: a };
                    ctx.alloc_expr(ArenaAnyExpr::ArenaView(view)) as &_
                },
            )
            .labelled("call");

        // Tuples: comma at the lowest of the high-precedence level, left-assoc
        let tuple = call
            .clone()
            .foldl(
                just(Token::Comma).ignore_then(call.clone()).repeated(),
                |a, b| {
                    let view = crate::expr::view::ExprView::Tuple(a, b);
                    ctx.alloc_expr(ArenaAnyExpr::ArenaView(view)) as &_
                },
            )
            .labelled("tuple");

        // Prefix not: ! binds looser than tuple/call
        let prefix = just(Token::Not).repeated().foldr(tuple.clone(), |_, rhs| {
            let view = crate::expr::view::ExprView::Not(rhs);
            ctx.alloc_expr(ArenaAnyExpr::ArenaView(view)) as &_
        });

        // Equality: left-assoc
        let equal = prefix
            .clone()
            .foldl(
                just(Token::Equal).ignore_then(prefix.clone()).repeated(),
                |a, b| {
                    let view = crate::expr::view::ExprView::Equal(a, b);
                    ctx.alloc_expr(ArenaAnyExpr::ArenaView(view)) as &_
                },
            )
            .labelled("equality");

        // And/Or/Iff at same precedence, left-assoc
        #[derive(Clone, Copy)]
        enum LOp {
            And,
            Or,
            Iff,
        }
        let lop = choice((
            just(Token::And).to(LOp::And),
            just(Token::Or).to(LOp::Or),
            just(Token::Iff).to(LOp::Iff),
        ));
        let logic_non_impl = equal
            .clone()
            .foldl(lop.then(equal).repeated(), |a, (op, b)| match op {
                LOp::And => {
                    let view = crate::expr::view::ExprView::And(a, b);
                    ctx.alloc_expr(ArenaAnyExpr::ArenaView(view)) as &_
                }
                LOp::Or => {
                    let view = crate::expr::view::ExprView::Or(a, b);
                    ctx.alloc_expr(ArenaAnyExpr::ArenaView(view)) as &_
                }
                LOp::Iff => {
                    let view = crate::expr::view::ExprView::Iff(a, b);
                    ctx.alloc_expr(ArenaAnyExpr::ArenaView(view)) as &_
                }
            })
            .labelled("logic");

        // Implies is right-assoc
        let implies = recursive(|imp| {
            logic_non_impl
                .clone()
                .then(just(Token::Implies).ignore_then(imp).or_not())
                .map(|(a, b)| match b {
                    Some(b) => {
                        let view = crate::expr::view::ExprView::Implies(a, b);
                        ctx.alloc_expr(ArenaAnyExpr::ArenaView(view)) as &_
                    }
                    None => a,
                })
                .labelled("implies")
        });

        // Lambda: right-assoc, looser than logic
        let lambda = recursive(|lam| {
            implies
                .clone()
                .then(just(Token::Arrow).ignore_then(lam).or_not())
                .map(|(a, b)| match b {
                    Some(b) => {
                        let view = crate::expr::view::ExprView::Lambda { arg: a, body: b };
                        ctx.alloc_expr(ArenaAnyExpr::ArenaView(view)) as &_
                    }
                    None => a,
                })
                .labelled("lambda")
        });

        // Quantifiers & If wrap full lambda-level expressions
        let quant = choice((
            just(Token::ForAll)
                .ignore_then(ident)
                .then_ignore(just(Token::Colon))
                .then(lambda.clone())
                .then_ignore(just(Token::Dot))
                .then(expr.clone())
                .map(|((v, dt), inner)| {
                    let view = crate::expr::view::ExprView::Forall {
                        variable: v,
                        dtype: dt,
                        inner,
                    };
                    ctx.alloc_expr(ArenaAnyExpr::ArenaView(view)) as &_
                }),
            just(Token::Exists)
                .ignore_then(ident)
                .then_ignore(just(Token::Colon))
                .then(lambda.clone())
                .then_ignore(just(Token::Dot))
                .then(expr.clone())
                .map(|((v, dt), inner)| {
                    let view = crate::expr::view::ExprView::Exists {
                        variable: v,
                        dtype: dt,
                        inner,
                    };
                    ctx.alloc_expr(ArenaAnyExpr::ArenaView(view)) as &_
                }),
        ))
        .labelled("quantifier");

        let if_ = just(Token::If)
            .ignore_then(lambda.clone())
            .then_ignore(just(Token::Then))
            .then(lambda.clone())
            .then_ignore(just(Token::Else))
            .then(lambda.clone())
            .map(|((cond, th), el)| {
                let view = crate::expr::view::ExprView::If {
                    condition: cond,
                    then_branch: th,
                    else_branch: el,
                };
                ctx.alloc_expr(ArenaAnyExpr::ArenaView(view)) as &_
            })
            .labelled("if-expression");

        // Top-level expression preference: if/quantifiers or plain lambda-level expr
        if_.or(quant).or(lambda)
    })
}

// ---------------- Public API ----------------

/// Parse a pretty-printed unified expression into an [`AnyExpr`](crate::expr::AnyExpr).
///
/// Returns `Ok(AnyExpr)` on success, or `Err(Vec<String>)` with human-readable diagnostics.
///
/// Complexity
/// - Lexing and parsing are linear in input size on typical code; error recovery may explore
///   limited alternatives.
///
/// Example
/// ```
/// use hyformal::parser::parse;
/// use hyformal::expr::Expr;
/// let e = parse("forall %0 : Bool . %0 = %0").unwrap();
/// assert_eq!(e.as_ref().view().type_(), hyformal::expr::variant::ExprType::Forall);
/// ```
pub fn parse(src: &str) -> Result<AnyExpr, Vec<String>> {
    // 1) Lexing (unchanged)
    let (tokens, lex_errs) = lexer().parse(src).into_output_errors();
    let mut errors: Vec<String> = Vec::new();
    errors.extend(lex_errs.into_iter().map(|e| format!("lexing error: {e}")));

    let tokens = match tokens {
        Some(toks) => toks,
        None => return Err(errors),
    };

    // 2) Parsing with chumsky over the token stream, building arena-backed exprs
    // Convert to a plain token stream for the parser (we keep spans only for lexing)
    let ctx = ExprArenaCtx::new();
    let plain: Vec<Token> = tokens.iter().map(|(t, _s)| t.clone()).collect();
    let (root_ref, parse_errs) = arena_expr_parser(&ctx)
        .then_ignore(end())
        .parse(plain.as_slice())
        .into_output_errors();
    errors.extend(
        parse_errs
            .into_iter()
            .map(|e| format!("parse error: {e:?}. Input tokens: {plain:?}")),
    );
    if !errors.is_empty() {
        return Err(errors);
    }

    let root_ref = match root_ref {
        Some(a) => a,
        None => return Err(errors),
    };

    // 3) Encode to AnyExpr via the arena-backed expression
    Ok(root_ref.encode())
}

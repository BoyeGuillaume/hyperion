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
use chumsky::{input::ValueInput, prelude::*};
use typed_arena::Arena;

use crate::encoding::tree::{TreeBuf, TreeBufNodeRef};
use crate::expr::AnyExpr;
use crate::expr::variant::ExprType;
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

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
            Token::Comma => write!(f, ","),
            Token::Colon => write!(f, ":"),
            Token::Dot => write!(f, "."),
            Token::Not => write!(f, "!"),
            Token::And => write!(f, "/\\"),
            Token::Or => write!(f, "\\/"),
            Token::Implies => write!(f, "=>"),
            Token::Iff => write!(f, "<=>"),
            Token::Equal => write!(f, "="),
            Token::Arrow => write!(f, "->"),
            Token::NeverSym => write!(f, "<>"),
            Token::If => write!(f, "if"),
            Token::Then => write!(f, "then"),
            Token::Else => write!(f, "else"),
            Token::ForAll => write!(f, "forall"),
            Token::Exists => write!(f, "exists"),
            Token::True => write!(f, "true"),
            Token::False => write!(f, "false"),
            Token::Bool => write!(f, "Bool"),
            Token::Omega => write!(f, "Omega"),
            Token::Powerset => write!(f, "Powerset"),
            Token::Var(v) => write!(f, "{v}"),
        }
    }
}

// ---------------- Lexer ----------------

fn char_to_id(c: char) -> u32 {
    let lc = c.to_ascii_lowercase();
    if lc.is_ascii_lowercase() {
        (lc as u8 - b'a') as u32
    } else {
        panic!("Invalid variable character: {c}");
    }
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
    let word = any()
        .filter(|c: &char| c.is_ascii_alphabetic() || *c == '_')
        .then(
            any()
                .filter(|c: &char| c.is_ascii_alphanumeric() || *c == '_')
                .repeated(),
        )
        .to_slice()
        .try_map(|s: &str, span| -> Result<Token, Rich<char>> {
            // Keywords
            let tok = match s {
                "if" => Token::If,
                "then" => Token::Then,
                "else" => Token::Else,
                "forall" => Token::ForAll,
                "exists" => Token::Exists,
                "true" => Token::True,
                "false" => Token::False,
                "Bool" => Token::Bool,
                "Omega" => Token::Omega,
                "Powerset" => Token::Powerset,
                _ => {
                    // Variables
                    if s.len() == 1 && s.chars().next().unwrap().is_ascii_alphabetic() {
                        Token::Var(InlineVariable::new_from_raw(char_to_id(
                            s.chars().next().unwrap(),
                        )))
                    } else if s.starts_with('v') || s.starts_with('_') {
                        let num_part = &s[1..];
                        if !num_part.is_empty() && num_part.chars().all(|c| c.is_ascii_digit()) {
                            let id: u32 = num_part.parse().unwrap();
                            Token::Var(InlineVariable::new_from_raw(id + 26))
                        } else {
                            return Err(Rich::custom(
                                span,
                                format!(
                                    "invalid variable '{s}': expected 'v<number>' or '_<number>' (e.g., v0, _42)"
                                ),
                            ));
                        }
                    } else {
                        return Err(Rich::custom(
                            span,
                            format!(
                                "unrecognized identifier '{s}': expected a variable name like a, X, v<number>, or _<number>"
                            ),
                        ));
                    }
                }
            };
            Ok(tok)
        });

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
        iff, implies, arrow, never, and_op, or_op, // Keywords and identifiers
        word,  // Punct/single char ops
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

// ---------------- Owned AST (for building via chumsky) ----------------

#[derive(Debug, Clone)]
enum Ast<'a> {
    // Term-level
    Var(InlineVariable),
    Call {
        func: &'a Ast<'a>,
        arg: &'a Ast<'a>,
    },
    If {
        condition: &'a Ast<'a>,
        then_branch: &'a Ast<'a>,
        else_branch: &'a Ast<'a>,
    },
    Tuple(&'a Ast<'a>, &'a Ast<'a>),
    Lambda {
        arg: &'a Ast<'a>,
        body: &'a Ast<'a>,
    },

    // Logic-level
    True,
    False,
    Not(&'a Ast<'a>),
    And(&'a Ast<'a>, &'a Ast<'a>),
    Or(&'a Ast<'a>, &'a Ast<'a>),
    Implies(&'a Ast<'a>, &'a Ast<'a>),
    Iff(&'a Ast<'a>, &'a Ast<'a>),
    ForAll {
        variable: InlineVariable,
        dtype: &'a Ast<'a>,
        inner: &'a Ast<'a>,
    },
    Exists {
        variable: InlineVariable,
        dtype: &'a Ast<'a>,
        inner: &'a Ast<'a>,
    },
    Equal(&'a Ast<'a>, &'a Ast<'a>),

    // Type-level
    Bool,
    Omega,
    Never,
    Powerset(&'a Ast<'a>),
    // Error placeholder (used by recovery) – encodes as Never
    Error,
}

impl<'a> Ast<'a> {
    fn encode_into_tree(&self, tree: &mut TreeBuf) -> TreeBufNodeRef {
        use ExprType::*;
        match self {
            // Term-level and misc
            Ast::Var(v) => tree.push_node(Variable as u8, Some(v.raw()), &[]),
            Ast::Call { func, arg } => {
                let f = func.encode_into_tree(tree);
                let a = arg.encode_into_tree(tree);
                tree.push_node(Call as u8, None, &[f, a])
            }
            Ast::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let c = condition.encode_into_tree(tree);
                let t = then_branch.encode_into_tree(tree);
                let e = else_branch.encode_into_tree(tree);
                tree.push_node(If as u8, None, &[c, t, e])
            }
            Ast::Tuple(a, b) => {
                let x = a.encode_into_tree(tree);
                let y = b.encode_into_tree(tree);
                tree.push_node(Tuple as u8, None, &[x, y])
            }
            Ast::Lambda { arg, body } => {
                let a = arg.encode_into_tree(tree);
                let b = body.encode_into_tree(tree);
                tree.push_node(Lambda as u8, None, &[a, b])
            }

            // Logic-level
            Ast::True => tree.push_node(True as u8, None, &[]),
            Ast::False => tree.push_node(False as u8, None, &[]),
            Ast::Not(p) => {
                let i = p.encode_into_tree(tree);
                tree.push_node(Not as u8, None, &[i])
            }
            Ast::And(a, b) => {
                let x = a.encode_into_tree(tree);
                let y = b.encode_into_tree(tree);
                tree.push_node(And as u8, None, &[x, y])
            }
            Ast::Or(a, b) => {
                let x = a.encode_into_tree(tree);
                let y = b.encode_into_tree(tree);
                tree.push_node(Or as u8, None, &[x, y])
            }
            Ast::Implies(a, b) => {
                let x = a.encode_into_tree(tree);
                let y = b.encode_into_tree(tree);
                tree.push_node(Implies as u8, None, &[x, y])
            }
            Ast::Iff(a, b) => {
                let x = a.encode_into_tree(tree);
                let y = b.encode_into_tree(tree);
                tree.push_node(Iff as u8, None, &[x, y])
            }
            Ast::ForAll {
                variable,
                dtype,
                inner,
            } => {
                let dt = dtype.encode_into_tree(tree);
                let inn = inner.encode_into_tree(tree);
                tree.push_node(Forall as u8, Some(variable.raw()), &[dt, inn])
            }
            Ast::Exists {
                variable,
                dtype,
                inner,
            } => {
                let dt = dtype.encode_into_tree(tree);
                let inn = inner.encode_into_tree(tree);
                tree.push_node(Exists as u8, Some(variable.raw()), &[dt, inn])
            }
            Ast::Equal(a, b) => {
                let x = a.encode_into_tree(tree);
                let y = b.encode_into_tree(tree);
                tree.push_node(Equal as u8, None, &[x, y])
            }

            // Type-level
            Ast::Bool => tree.push_node(Bool as u8, None, &[]),
            Ast::Omega => tree.push_node(Omega as u8, None, &[]),
            Ast::Never => tree.push_node(Never as u8, None, &[]),
            Ast::Powerset(a) => {
                let x = a.encode_into_tree(tree);
                tree.push_node(Powerset as u8, None, &[x])
            }
            Ast::Error => panic!("Should not encode Error AST node"),
        }
    }
}

// ---------------- chumsky parser over tokens ----------------

fn ast_parser<'tokens, I>(
    arena: &'tokens mut Arena<Ast<'tokens>>,
) -> impl Parser<'tokens, I, Ast<'tokens>, extra::Err<Rich<'tokens, Token, Span>>> + Clone
where
    I: ValueInput<'tokens, Token = Token, Span = Span>,
{
    recursive(|expr| {
        // Identifiers/values
        let ident = select! { Token::Var(v) => v };

        let value = select! {
            Token::True => Ast::True,
            Token::False => Ast::False,
            Token::Bool => Ast::Bool,
            Token::Omega => Ast::Omega,
            Token::NeverSym => Ast::Never,
        };

        // Parenthesised expressions (recover mismatched/missing parens)
        let paren_expr = expr
            .clone()
            .delimited_by(just(Token::LParen), just(Token::RParen))
            .recover_with(via_parser(nested_delimiters(
                Token::LParen,
                Token::RParen,
                [],
                |_| Ast::Error,
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
                        |_| Ast::Error,
                    )))
                    .labelled("powerset-args"),
            )
            .map(|e| Ast::Powerset(arena.alloc(e)))
            .labelled("Powerset");

        // Atomic expressions (no ambiguity)
        let atom = value
            .or(ident.map(Ast::Var))
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
                        |_| Ast::Error,
                    )))
                    .labelled("call-args")
                    .repeated(),
                |f, a| Ast::Call {
                    func: arena.alloc(f),
                    arg: arena.alloc(a),
                },
            )
            .labelled("call");

        // Tuples: comma at the lowest of the high-precedence level, left-assoc
        let tuple = call
            .clone()
            .foldl(
                just(Token::Comma).ignore_then(call.clone()).repeated(),
                |a, b| Ast::Tuple(arena.alloc(a), arena.alloc(b)),
            )
            .labelled("tuple");

        // Prefix not: ! binds looser than tuple/call
        let prefix = just(Token::Not)
            .repeated()
            .foldr(tuple.clone(), |_, rhs| Ast::Not(arena.alloc(rhs)));

        // Equality: left-assoc
        let equal = prefix
            .clone()
            .foldl(
                just(Token::Equal).ignore_then(prefix.clone()).repeated(),
                |a, b| Ast::Equal(arena.alloc(a), arena.alloc(b)),
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
                LOp::And => Ast::And(arena.alloc(a), arena.alloc(b)),
                LOp::Or => Ast::Or(arena.alloc(a), arena.alloc(b)),
                LOp::Iff => Ast::Iff(arena.alloc(a), arena.alloc(b)),
            })
            .labelled("logic");

        // Implies is right-assoc
        let implies = recursive(|imp| {
            logic_non_impl
                .clone()
                .then(just(Token::Implies).ignore_then(imp).or_not())
                .map(|(a, b)| match b {
                    Some(b) => Ast::Implies(arena.alloc(a), arena.alloc(b)),
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
                    Some(b) => Ast::Lambda {
                        arg: arena.alloc(a),
                        body: arena.alloc(b),
                    },
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
                .map(|((v, dt), inner)| Ast::ForAll {
                    variable: v,
                    dtype: arena.alloc(dt),
                    inner: arena.alloc(inner),
                }),
            just(Token::Exists)
                .ignore_then(ident)
                .then_ignore(just(Token::Colon))
                .then(lambda.clone())
                .then_ignore(just(Token::Dot))
                .then(expr.clone())
                .map(|((v, dt), inner)| Ast::Exists {
                    variable: v,
                    dtype: arena.alloc(dt),
                    inner: arena.alloc(inner),
                }),
        ))
        .labelled("quantifier");

        let if_ = just(Token::If)
            .ignore_then(lambda.clone())
            .then_ignore(just(Token::Then))
            .then(lambda.clone())
            .then_ignore(just(Token::Else))
            .then(lambda.clone())
            .map(|((cond, th), el)| Ast::If {
                condition: arena.alloc(cond),
                then_branch: arena.alloc(th),
                else_branch: arena.alloc(el),
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
/// let e = parse("forall x : Bool . x = x").unwrap();
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

    // 2) Parsing with chumsky over the token stream
    // Convert to a plain token stream for the parser (we keep spans only for lexing)
    let mut arena = Arena::new();
    let plain: Vec<Token> = tokens.iter().map(|(t, _s)| t.clone()).collect();
    let (ast, parse_errs) = ast_parser(&mut arena)
        .then_ignore(end())
        .parse(plain.as_slice())
        .into_output_errors();
    errors.extend(parse_errs.into_iter().map(|e| format!("parse error: {e}")));
    if !errors.is_empty() {
        return Err(errors);
    }

    let ast = match ast {
        Some(a) => a,
        None => return Err(errors),
    };

    // 3) Encode to AnyExpr via TreeBuf
    let mut tree = TreeBuf::new();
    let root = ast.encode_into_tree(&mut tree);
    tree.set_root(root);
    tree.consolite_if_needed();
    Ok(AnyExpr { tree })
}

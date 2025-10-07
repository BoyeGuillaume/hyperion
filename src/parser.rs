//! Parser for the pretty-printed unified expression language using chumsky.
//!
//! Two stages:
//! 1) Tokenisation from input string to a `Token` stream.
//! 2) Parsing tokens into a lightweight arena-allocated AST, then encoding to `DynExpr`.
//!
//! The accepted syntax is designed to round-trip with the pretty-printer in
//! `expr::pretty`:
//! - Variables: single letter a..z or A..Z map to ids 0..25. Larger ids: `v<number>`
//!   (primary) or `_number` (also accepted) map to that raw id.
//! - Literals/types: true, false, Bool, Omega, `<>` (Never), `Powerset(expr)`.
//! - Application: `f(expr)` where `f` is a variable.
//! - Tuples: `A, B` (comma is the lowest-precedence operator; left-associative).
//! - Function types: `A -> B` (right-associative).
//! - Conditionals: `if P then X else Y`.
//! - Logic: `!P`, `P /\ Q`, `P \/ Q`, `P => Q`, `P <=> Q`, `A = B`.
//! - Quantifiers: `forall x : T . P` and `exists x : T . P`.
//!
//! Note: The pretty-printer inserts parentheses when operators of the same precedence
//! differ; this parser accepts those forms and also a reasonable precedence hierarchy.
use chumsky::prelude::*;

use crate::encoding::{DynBuf, integer, magic};
use crate::expr::DynExpr;
use crate::variable::InlineVariable;
use typed_arena::Arena;

pub type Spanned<T> = (T, SimpleSpan);

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
    Arrow,    // -> (types)
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

    // Error (misc)
    Error,
}

// ---------------- Lexer ----------------

fn char_to_id(c: char) -> u64 {
    if c.is_ascii_lowercase() {
        (c as u8 - b'a') as u64
    } else if c.is_ascii_uppercase() {
        (c as u8 - b'A' + 26) as u64
    } else {
        panic!("Invalid variable character: {}", c);
    }
}

fn lexer<'a>() -> impl Parser<'a, &'a str, Vec<Spanned<Token>>, extra::Err<Simple<'a, char>>> {
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
        .filter(|c: &char| c.is_ascii() || *c == '_')
        .then(
            any()
                .filter(|c: &char| c.is_ascii_alphanumeric() || *c == '_')
                .repeated(),
        )
        .to_slice()
        .map(|s: &str| match s {
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
            s => {
                // Attempt to match either a short var or long var
                if s.len() == 1 && s.chars().next().unwrap().is_ascii_alphabetic() {
                    Token::Var(InlineVariable::new_from_raw(char_to_id(
                        s.chars().next().unwrap(),
                    )))
                } else if s.starts_with('v') || s.starts_with('_') {
                    let num_part = &s[1..];
                    match num_part.parse::<u64>() {
                        Ok(id) => Token::Var(InlineVariable::new_from_raw(id + 26)),
                        _ => Token::Error,
                    }
                } else {
                    Token::Error
                }
            }
        })
        .filter(|t| *t != Token::Error);

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

// ---------------- Arena AST ----------------

#[derive(Debug)]
enum Ast<'a> {
    // Term-level
    Var(InlineVariable),
    App {
        func: InlineVariable,
        arg: &'a Ast<'a>,
    },
    If {
        condition: &'a Ast<'a>,
        then_branch: &'a Ast<'a>,
        else_branch: &'a Ast<'a>,
    },
    Tuple(&'a Ast<'a>, &'a Ast<'a>),

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
    Func(&'a Ast<'a>, &'a Ast<'a>),
}

impl<'a> Ast<'a> {
    fn encode_into<F: FnMut(&[u8])>(&self, f: &mut F) -> u64 {
        use magic::*;
        match self {
            // Term-level
            Ast::Var(v) => {
                let mut sz = 0;
                sz += integer::encode_u64(v.raw(), f);
                f(&[MISC_VAR]);
                sz + 1
            }
            Ast::App { func, arg } => {
                let mut sz = 0;
                sz += (*arg).encode_into(f);
                sz += integer::encode_u64(func.raw(), f);
                f(&[E_APP]);
                sz + 1
            }
            Ast::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let mut sz = 0;
                sz += (*condition).encode_into(f);
                let then_len = (*then_branch).encode_into(f);
                sz += then_len;
                let else_len = (*else_branch).encode_into(f);
                sz += else_len;
                sz += integer::encode_u64(else_len, f);
                sz += integer::encode_u64(then_len, f);
                f(&[E_IF]);
                sz + 1
            }
            Ast::Tuple(a, b) => {
                let mut sz = 0;
                sz += (*a).encode_into(f);
                let r = (*b).encode_into(f);
                sz += r;
                sz += integer::encode_u64(r, f);
                f(&[E_TUPLE]);
                sz + 1
            }

            // Logic-level
            Ast::True => {
                f(&[P_TRUE]);
                1
            }
            Ast::False => {
                f(&[P_FALSE]);
                1
            }
            Ast::Not(p) => {
                let s = (*p).encode_into(f);
                f(&[P_NOT]);
                s + 1
            }
            Ast::And(a, b) => {
                let mut s = 0;
                s += (*a).encode_into(f);
                let r = (*b).encode_into(f);
                s += r;
                s += integer::encode_u64(r, f);
                f(&[P_AND]);
                s + 1
            }
            Ast::Or(a, b) => {
                let mut s = 0;
                s += (*a).encode_into(f);
                let r = (*b).encode_into(f);
                s += r;
                s += integer::encode_u64(r, f);
                f(&[P_OR]);
                s + 1
            }
            Ast::Implies(a, b) => {
                let mut s = 0;
                s += (*a).encode_into(f);
                let r = (*b).encode_into(f);
                s += r;
                s += integer::encode_u64(r, f);
                f(&[P_IMPLIES]);
                s + 1
            }
            Ast::Iff(a, b) => {
                let mut s = 0;
                s += (*a).encode_into(f);
                let r = (*b).encode_into(f);
                s += r;
                s += integer::encode_u64(r, f);
                f(&[P_IFF]);
                s + 1
            }
            Ast::ForAll {
                variable,
                dtype,
                inner,
            } => {
                let mut s = 0;
                s += (*dtype).encode_into(f);
                let r = (*inner).encode_into(f);
                s += r;
                s += integer::encode_u64(r, f);
                s += integer::encode_u64(variable.raw(), f);
                f(&[P_FORALL]);
                s + 1
            }
            Ast::Exists {
                variable,
                dtype,
                inner,
            } => {
                let mut s = 0;
                s += (*dtype).encode_into(f);
                let r = (*inner).encode_into(f);
                s += r;
                s += integer::encode_u64(r, f);
                s += integer::encode_u64(variable.raw(), f);
                f(&[P_EXISTS]);
                s + 1
            }
            Ast::Equal(a, b) => {
                let mut s = 0;
                s += (*a).encode_into(f);
                let r = (*b).encode_into(f);
                s += r;
                s += integer::encode_u64(r, f);
                f(&[P_EQUAL]);
                s + 1
            }

            // Type-level
            Ast::Bool => {
                f(&[T_BOOL]);
                1
            }
            Ast::Omega => {
                f(&[T_OMEGA]);
                1
            }
            Ast::Never => {
                f(&[E_NEVER]);
                1
            }
            Ast::Powerset(a) => {
                let s = (*a).encode_into(f);
                f(&[T_POWER]);
                s + 1
            }
            Ast::Func(a, b) => {
                let mut s = 0;
                s += (*a).encode_into(f);
                let r = (*b).encode_into(f);
                s += r;
                s += integer::encode_u64(r, f);
                f(&[T_FUNC]);
                s + 1
            }
        }
    }
}

// ---------------- Hand-rolled recursive-descent parser over tokens ----------------

struct TS<'a> {
    toks: &'a [Spanned<Token>],
    pos: usize,
}

impl<'a> TS<'a> {
    fn new(toks: &'a [Spanned<Token>]) -> Self {
        TS { toks, pos: 0 }
    }
    fn peek(&self) -> Option<&Spanned<Token>> {
        self.toks.get(self.pos)
    }
    fn next(&mut self) -> Option<&Spanned<Token>> {
        let r = self.toks.get(self.pos);
        if r.is_some() {
            self.pos += 1;
        }
        r
    }
    fn eat(&mut self, t: &Token) -> bool {
        if let Some((tok, _)) = self.peek() {
            if tok == t {
                self.pos += 1;
                return true;
            }
        }
        false
    }
}

// Primary atoms: variables, literals, parenthesised full expressions, and
// Powerset(...) constructor. Parentheses are treated as grouping for the full
// expression (parse_if) so inner precedence is handled correctly.
fn parse_primary<'a>(ts: &mut TS<'a>, arena: &'a Arena<Ast<'a>>) -> Result<&'a Ast<'a>, String> {
    match ts.next() {
        Some((Token::LParen, _)) => {
            let e = parse_if(ts, arena)?;
            if !ts.eat(&Token::RParen) {
                return Err("expected ')'".into());
            }
            Ok(e)
        }
        Some((Token::True, _)) => Ok(arena.alloc(Ast::True)),
        Some((Token::False, _)) => Ok(arena.alloc(Ast::False)),
        Some((Token::Bool, _)) => Ok(arena.alloc(Ast::Bool)),
        Some((Token::Omega, _)) => Ok(arena.alloc(Ast::Omega)),
        Some((Token::NeverSym, _)) => Ok(arena.alloc(Ast::Never)),
        Some((Token::Powerset, _)) => {
            if !ts.eat(&Token::LParen) {
                return Err("expected '(' after Powerset".into());
            }
            let inner = parse_if(ts, arena)?;
            if !ts.eat(&Token::RParen) {
                return Err("expected ')' after Powerset(arg)".into());
            }
            Ok(arena.alloc(Ast::Powerset(inner)))
        }
        Some((Token::Var(f), _)) => Ok(arena.alloc(Ast::Var(*f))),
        Some((tok, _)) => Err(format!("unexpected token in primary: {:?}", tok)),
        None => Err("unexpected end of input".into()),
    }
}

// Function arrow (->) is a tight infix operator and is right-associative.
fn parse_func<'a>(ts: &mut TS<'a>, arena: &'a Arena<Ast<'a>>) -> Result<&'a Ast<'a>, String> {
    let lhs = parse_primary(ts, arena)?;
    if matches!(ts.peek(), Some((Token::Arrow, _))) {
        ts.next();
        let rhs = parse_func(ts, arena)?;
        Ok(arena.alloc(Ast::Func(lhs, rhs)))
    } else {
        Ok(lhs)
    }
}

// Application level: application (f(arg)) binds tighter than Not and logic.
// We only allow application when the callee is a Var (as in the AST shape).
fn parse_app<'a>(ts: &mut TS<'a>, arena: &'a Arena<Ast<'a>>) -> Result<&'a Ast<'a>, String> {
    let mut e = parse_func(ts, arena)?;
    loop {
        // application form: Var '(' expr ')'
        if let Some((Token::LParen, _)) = ts.peek() {
            match e {
                Ast::Var(v) => {
                    ts.next();
                    let arg = parse_if(ts, arena)?;
                    if !ts.eat(&Token::RParen) {
                        return Err("expected ')' after call".into());
                    }
                    e = arena.alloc(Ast::App { func: *v, arg });
                }
                _ => break,
            }
        } else {
            break;
        }
    }
    Ok(e)
}

// Tuple level: comma-separated expressions, left-associative, built on app level.
fn parse_tuple<'a>(ts: &mut TS<'a>, arena: &'a Arena<Ast<'a>>) -> Result<&'a Ast<'a>, String> {
    let mut e = parse_app(ts, arena)?;
    while matches!(ts.peek(), Some((Token::Comma, _))) {
        ts.next();
        let rhs = parse_app(ts, arena)?;
        e = arena.alloc(Ast::Tuple(e, rhs));
    }
    Ok(e)
}

// parse_prefix: handles leading `!` operators and wraps tuple-level expressions.
fn parse_prefix<'a>(ts: &mut TS<'a>, arena: &'a Arena<Ast<'a>>) -> Result<&'a Ast<'a>, String> {
    let mut count = 0;
    while matches!(ts.peek(), Some((Token::Not, _))) {
        ts.next();
        count += 1;
    }
    let mut e = parse_tuple(ts, arena)?;
    for _ in 0..count {
        e = arena.alloc(Ast::Not(e));
    }
    Ok(e)
}

// Equality level sits above prefix/app/tuple and below logic-level operators.
fn parse_equal<'a>(ts: &mut TS<'a>, arena: &'a Arena<Ast<'a>>) -> Result<&'a Ast<'a>, String> {
    let mut e = parse_prefix(ts, arena)?;
    while matches!(ts.peek(), Some((Token::Equal, _))) {
        ts.next();
        let rhs = parse_prefix(ts, arena)?;
        e = arena.alloc(Ast::Equal(e, rhs));
    }
    Ok(e)
}

// Combined logic-level: And/Or/Iff at the same precedence, Implies right-assoc.
fn parse_logic<'a>(ts: &mut TS<'a>, arena: &'a Arena<Ast<'a>>) -> Result<&'a Ast<'a>, String> {
    let mut e = parse_equal(ts, arena)?;
    loop {
        match ts.peek() {
            Some((Token::And, _)) => {
                ts.next();
                let rhs = parse_equal(ts, arena)?;
                e = arena.alloc(Ast::And(e, rhs));
            }
            Some((Token::Or, _)) => {
                ts.next();
                let rhs = parse_equal(ts, arena)?;
                e = arena.alloc(Ast::Or(e, rhs));
            }
            Some((Token::Implies, _)) => {
                ts.next();
                let rhs = parse_logic(ts, arena)?;
                return Ok(arena.alloc(Ast::Implies(e, rhs)));
            }
            Some((Token::Iff, _)) => {
                ts.next();
                let rhs = parse_equal(ts, arena)?;
                e = arena.alloc(Ast::Iff(e, rhs));
            }
            _ => break,
        }
    }
    Ok(e)
}

// Top-level: If and quantifiers have the lowest precedence on the term side
// (they wrap full expressions). parse_if handles `if ... then ... else ...` and
// delegates to parse_logic for condition/branches; quantifiers are similar.
fn parse_if<'a>(ts: &mut TS<'a>, arena: &'a Arena<Ast<'a>>) -> Result<&'a Ast<'a>, String> {
    if matches!(ts.peek(), Some((Token::If, _))) {
        ts.next();
        let cond = parse_logic(ts, arena)?;
        if !ts.eat(&Token::Then) {
            return Err("expected 'then'".into());
        }
        let th = parse_logic(ts, arena)?;
        if !ts.eat(&Token::Else) {
            return Err("expected 'else'".into());
        }
        let el = parse_logic(ts, arena)?;
        Ok(arena.alloc(Ast::If {
            condition: cond,
            then_branch: th,
            else_branch: el,
        }))
    } else if matches!(ts.peek(), Some((Token::ForAll, _))) {
        ts.next();
        let v = match ts.next() {
            Some((Token::Var(v), _)) => *v,
            _ => return Err("expected variable after 'forall'".into()),
        };
        if !ts.eat(&Token::Colon) {
            return Err("expected ':' after variable".into());
        }
        let dt = parse_logic(ts, arena)?;
        if !ts.eat(&Token::Dot) {
            return Err("expected '.' after domain".into());
        }
        let inner = parse_logic(ts, arena)?;
        Ok(arena.alloc(Ast::ForAll {
            variable: v,
            dtype: dt,
            inner,
        }))
    } else if matches!(ts.peek(), Some((Token::Exists, _))) {
        ts.next();
        let v = match ts.next() {
            Some((Token::Var(v), _)) => *v,
            _ => return Err("expected variable after 'exists'".into()),
        };
        if !ts.eat(&Token::Colon) {
            return Err("expected ':' after variable".into());
        }
        let dt = parse_logic(ts, arena)?;
        if !ts.eat(&Token::Dot) {
            return Err("expected '.' after domain".into());
        }
        let inner = parse_logic(ts, arena)?;
        Ok(arena.alloc(Ast::Exists {
            variable: v,
            dtype: dt,
            inner,
        }))
    } else {
        // otherwise parse the next-lower precedence expression (logic level)
        parse_logic(ts, arena)
    }
}

// (helpers removed; integrated into expr_parser)

// ---------------- Public API ----------------

/// Parse a pretty-printed unified expression into a dynamically-encoded `DynExpr`.
/// Returns a `Result` with either the expression or a list of error strings.
pub fn parse(src: &str) -> Result<DynExpr, Vec<String>> {
    // 1) Lexing
    let (tokens, lex_errs) = lexer().parse(src).into_output_errors();
    let mut errors: Vec<String> = Vec::new();
    errors.extend(lex_errs.into_iter().map(|e: Simple<char>| e.to_string()));

    let tokens = match tokens {
        Some(toks) => toks,
        None => return Err(errors),
    };

    // 2) Parsing to arena AST (manual)
    let arena: Arena<Ast> = Arena::new();
    let mut ts = TS::new(tokens.as_slice());
    let ast = match parse_if(&mut ts, &arena) {
        Ok(a) => a,
        Err(msg) => {
            errors.push(msg);
            return Err(errors);
        }
    };
    if ts.peek().is_some() {
        errors.push("unexpected trailing tokens".into());
        return Err(errors);
    }

    // 3) Encode to DynExpr
    let mut buf = DynBuf::new();
    ast.encode_into(&mut |bytes| buf.extend_from_slice(bytes));
    Ok(DynExpr { bytes: buf })
}

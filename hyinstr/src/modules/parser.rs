use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    path::Path,
    rc::Rc,
    str::FromStr,
    sync::Arc,
};

use crate::analysis::{AnalysisStatistic, AnalysisStatisticOp, TerminationScope};
use bigdecimal::BigDecimal;
use chumsky::{
    container::Seq,
    extra::{ParserExtra, SimpleState},
    input::ValueInput,
    label::LabelError,
    prelude::*,
    text::Char,
};
use either::Either;
use log::{debug, error, warn};
use num_bigint::{BigInt, Sign};
use strum::{EnumDiscriminants, EnumIs, EnumTryAs, IntoEnumIterator};
use uuid::Uuid;

use crate::{
    consts::{AnyConst, fp::FConst, int::IConst},
    modules::{
        BasicBlock, CallingConvention, Function, Instruction, Module, Visibility,
        instructions::{
            HyInstr, HyInstrOp, InstructionFlags, fp::*, int::*, mem::*, meta::*, misc::*,
        },
        operand::{Label, Name, Operand},
        symbol::{FunctionPointer, FunctionPointerType},
        terminator::*,
    },
    types::{
        TypeRegistry, Typeref,
        aggregate::{ArrayType, StructType},
        primary::{FType, IType, PrimaryBasicType, PrimaryType, PtrType, VcSize, VcType},
    },
    utils::{Error, ParserError},
};

type Span = SimpleSpan;
type Spanned<T> = (T, Span);

#[derive(Debug, Clone, PartialEq, Eq, EnumIs, EnumTryAs, EnumDiscriminants)]
enum Token<'a> {
    // Special identifiers and keywords
    IType(IType),
    FType(FType),
    Ordering(MemoryOrdering),
    Visibility(Visibility),
    CallingConvention(CallingConvention),
    TerminatorOp(HyTerminatorOp),
    InstrOp(HyInstrOp, Vec<&'a str>),
    Void,
    Import,
    Identifier(&'a str, Vec<&'a str>),
    MetaIdentifier(&'a str, Vec<&'a str>), // Prefixed with '!'

    /// UUID parser (prefixed with '@')
    #[allow(dead_code)]
    Uuid(Uuid),

    /// Register identifier (prefixed with '%')
    Register(&'a str),

    /// Numeric literal (can be decimal, octal, hexadecimal or binary, prefixed accordingly)
    Number(BigInt),

    /// Decimal floating-point literal
    Decimal(BigDecimal),

    /// String literal (enclosed in double quotes)
    StringLiteral(String),

    /// Left parenthesis '('
    LParen,

    /// Right parenthesis ')'
    RParen,

    /// Left brace '{'
    LBrace,

    /// Right brace '}'
    RBrace,

    /// Left bracket '['
    LBracket,

    /// Right bracket ']'
    RBracket,

    /// Left angle bracket '<'
    LAngle,

    /// Right angle bracket '>'
    RAngle,

    /// Comma ','
    Comma,

    /// Colon ':'
    Colon,

    /// Equals '='
    Equals,

    /// Newline '\n'
    Newline,
}

impl std::fmt::Display for Token<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::IType(itype) => write!(f, "{}", itype),
            Token::FType(ftype) => write!(f, "{}", ftype),
            Token::Ordering(ordering) => write!(f, "{:?}", ordering),
            Token::Visibility(visibility) => write!(f, "{:?}", visibility),
            Token::CallingConvention(cc) => write!(f, "{:?}", cc),
            Token::TerminatorOp(op) => write!(f, "{:?}", op),
            Token::InstrOp(op, variants) => {
                if variants.is_empty() {
                    write!(f, "{:?}", op)
                } else {
                    write!(f, "{:?}<{:?}>", op, variants)
                }
            }
            Token::Void => write!(f, "void"),
            Token::Import => write!(f, "import"),
            Token::Identifier(s, variants) => {
                if variants.is_empty() {
                    write!(f, "{}", s)
                } else {
                    write!(f, "{}.{}", s, variants.join("."))
                }
            }
            Token::MetaIdentifier(s, variants) => {
                if variants.is_empty() {
                    write!(f, "!{}", s)
                } else {
                    write!(f, "!{}.{}", s, variants.join("."))
                }
            }
            Token::Uuid(uuid) => write!(f, "{}", uuid),
            Token::Register(name) => write!(f, "%{}", name),
            Token::Number(num) => write!(f, "{}", num),
            Token::Decimal(dec) => write!(f, "{}", dec),
            Token::StringLiteral(s) => write!(f, "{:?}", s),
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
            Token::LBrace => write!(f, "{{"),
            Token::RBrace => write!(f, "}}"),
            Token::LBracket => write!(f, "["),
            Token::RBracket => write!(f, "]"),
            Token::LAngle => write!(f, "<"),
            Token::RAngle => write!(f, ">"),
            Token::Comma => write!(f, ","),
            Token::Colon => write!(f, ":"),
            Token::Equals => write!(f, "="),
            Token::Newline => write!(f, "\\n"),
        }
    }
}

#[derive(Clone)]
struct State<'a> {
    label_namespace: BTreeMap<String, Label>,
    register_namespace: BTreeMap<String, Name>,
    func_retriever: Rc<dyn Fn(String, FunctionPointerType) -> Option<Uuid> + 'a>,
    uuid_generator: Rc<dyn Fn() -> Uuid + 'a>,
    type_registry: &'a TypeRegistry,
}

impl<'a> State<'a> {
    pub fn new(
        type_registry: &'a TypeRegistry,
        func_retriever: Rc<dyn Fn(String, FunctionPointerType) -> Option<Uuid> + 'a>,
        uuid_generator: Rc<dyn Fn() -> Uuid + 'a>,
    ) -> Self {
        Self {
            label_namespace: BTreeMap::new(),
            register_namespace: BTreeMap::new(),
            func_retriever,
            uuid_generator,
            type_registry,
        }
    }

    pub fn get_label(&mut self, name: &str) -> Label {
        if let Some(label) = self.label_namespace.get(name) {
            *label
        } else {
            let label = Label(self.label_namespace.len() as u32);
            self.label_namespace.insert(name.to_string(), label);
            label
        }
    }

    pub fn get_register(&mut self, name: &str) -> Name {
        if let Some(reg) = self.register_namespace.get(name) {
            *reg
        } else {
            let reg = Name(self.register_namespace.len() as u32);
            self.register_namespace.insert(name.to_string(), reg);
            reg
        }
    }

    pub fn clear(&mut self) {
        self.label_namespace.clear();
        self.register_namespace.clear();
    }
}

impl Token<'_> {
    pub fn discriminant(&self) -> TokenDiscriminants {
        self.into()
    }
}

macro_rules! fast_boxed {
    (
        $parser:expr
    ) => {{
        #[cfg(debug_assertions)]
        {
            $parser.boxed()
        }
        #[cfg(not(debug_assertions))]
        {
            $parser
        }
    }};
}

fn just_match<'src, I, E>(token: TokenDiscriminants) -> impl Parser<'src, I, Token<'src>, E> + Clone
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span> + Clone,
    E: ParserExtra<'src, I>,
    E::Error: LabelError<'src, I, String>,
{
    any()
        .filter(move |t: &Token| t.discriminant() == token)
        .labelled(format!("token {:?}", token))
}

#[allow(unused)]
fn uuid_parser<'src>() -> impl Parser<'src, &'src str, Uuid, extra::Err<Rich<'src, char>>> {
    // UUID parser in standard 8-4-4-4-12 format
    let hex_digit = any()
        .filter(|c: &char| c.is_ascii_hexdigit())
        .labelled("hexadecimal digit");
    just("@")
        .ignore_then(hex_digit)
        .repeated()
        .exactly(8)
        .then_ignore(just('-'))
        .then(hex_digit.repeated().exactly(4))
        .then_ignore(just('-'))
        .then(hex_digit.repeated().exactly(4))
        .then_ignore(just('-'))
        .then(hex_digit.repeated().exactly(4))
        .then_ignore(just('-'))
        .then(hex_digit.repeated().exactly(12))
        .to_slice()
        .validate(|s: &str, extra, emit| match uuid::Uuid::parse_str(s) {
            Ok(uuid) => uuid,
            Err(e) => {
                emit.emit(Rich::custom(
                    extra.span(),
                    format!("invalid UUID format: {}", e),
                ));
                uuid::Uuid::nil()
            }
        })
        .labelled("UUID")
}

fn string_parser<'src>()
-> impl Parser<'src, &'src str, String, extra::Err<Rich<'src, char>>> + Clone {
    just('"')
        .ignore_then(
            any()
                .filter(|&c: &char| c != '"' && c != '\\')
                .or(just('\\').ignore_then(any().map(|c| match c {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    '\\' => '\\',
                    '"' => '"',
                    other => other,
                })))
                .repeated()
                .collect::<String>(),
        )
        .then_ignore(just('"'))
        .labelled("string literal")
}

fn numeral_parser<'src>()
-> impl Parser<'src, &'src str, Token<'src>, extra::Err<Rich<'src, char>>> + Clone {
    let sign = just('+').or(just('-')).or_not();
    let decimal_digits = any()
        .filter(|c: &char| c.is_ascii_digit())
        .repeated()
        .at_least(1);
    let signed_exponent = (just('e').or(just('E')))
        .ignore_then(sign)
        .ignore_then(decimal_digits);

    let float_with_fraction = sign
        .ignore_then(decimal_digits)
        .then_ignore(just('.'))
        .then(decimal_digits)
        .then(signed_exponent.or_not())
        .to_slice()
        .validate(
            |literal: &str, extra, emit| match literal.parse::<BigDecimal>() {
                Ok(value) => Token::Decimal(value),
                Err(e) => {
                    emit.emit(Rich::custom(
                        extra.span(),
                        format!("invalid decimal literal '{}': {}", literal, e),
                    ));
                    Token::Decimal(BigDecimal::from(0))
                }
            },
        );

    let float_with_exponent = sign
        .ignore_then(decimal_digits)
        .then(signed_exponent)
        .to_slice()
        .validate(
            |literal: &str, extra, emit| match literal.parse::<BigDecimal>() {
                Ok(value) => Token::Decimal(value),
                Err(e) => {
                    emit.emit(Rich::custom(
                        extra.span(),
                        format!("invalid decimal literal '{}': {}", literal, e),
                    ));
                    Token::Decimal(BigDecimal::from(0))
                }
            },
        );

    let hex_int = sign
        .ignore_then(just("0x").or(just("0X")))
        .ignore_then(
            any()
                .filter(|c: &char| c.is_ascii_hexdigit())
                .repeated()
                .at_least(1),
        )
        .to_slice()
        .validate(|s: &str, extra, emit| {
            let (signum, rest) = if let Some(stripped) = s.strip_prefix('-') {
                (-1i32, stripped)
            } else if let Some(stripped) = s.strip_prefix('+') {
                (1i32, stripped)
            } else {
                (1i32, s)
            };

            let number_body = &rest[2..];
            match BigInt::parse_bytes(number_body.as_bytes(), 16) {
                Some(value) => Token::Number(if signum == -1 { -value } else { value }),
                None => {
                    emit.emit(Rich::custom(
                        extra.span(),
                        format!("invalid base 16 integer '{}'", s),
                    ));
                    Token::Number(BigInt::from(0))
                }
            }
        });

    let octal_int = sign
        .ignore_then(just("0o").or(just("0O")))
        .ignore_then(
            any()
                .filter(|c: &char| c.is_ascii_digit() && *c < '8')
                .repeated()
                .at_least(1),
        )
        .to_slice()
        .validate(|s: &str, extra, emit| {
            let (signum, rest) = if let Some(stripped) = s.strip_prefix('-') {
                (-1i32, stripped)
            } else if let Some(stripped) = s.strip_prefix('+') {
                (1i32, stripped)
            } else {
                (1i32, s)
            };

            let number_body = &rest[2..];
            match BigInt::parse_bytes(number_body.as_bytes(), 8) {
                Some(value) => Token::Number(if signum == -1 { -value } else { value }),
                None => {
                    emit.emit(Rich::custom(
                        extra.span(),
                        format!("invalid base 8 integer '{}'", s),
                    ));
                    Token::Number(BigInt::from(0))
                }
            }
        });

    let binary_int = sign
        .ignore_then(just("0b").or(just("0B")))
        .ignore_then(
            any()
                .filter(|c: &char| *c == '0' || *c == '1')
                .repeated()
                .at_least(1),
        )
        .to_slice()
        .validate(|s: &str, extra, emit| {
            let (signum, rest) = if let Some(stripped) = s.strip_prefix('-') {
                (-1i32, stripped)
            } else if let Some(stripped) = s.strip_prefix('+') {
                (1i32, stripped)
            } else {
                (1i32, s)
            };

            let number_body = &rest[2..];
            match BigInt::parse_bytes(number_body.as_bytes(), 2) {
                Some(value) => Token::Number(if signum == -1 { -value } else { value }),
                None => {
                    emit.emit(Rich::custom(
                        extra.span(),
                        format!("invalid base 2 integer '{}'", s),
                    ));
                    Token::Number(BigInt::from(0))
                }
            }
        });

    let decimal_int =
        sign.ignore_then(decimal_digits)
            .to_slice()
            .validate(|s: &str, extra, emit| {
                let (signum, rest) = if let Some(stripped) = s.strip_prefix('-') {
                    (-1i32, stripped)
                } else if let Some(stripped) = s.strip_prefix('+') {
                    (1i32, stripped)
                } else {
                    (1i32, s)
                };

                let number_body = rest;
                match BigInt::parse_bytes(number_body.as_bytes(), 10) {
                    Some(value) => Token::Number(if signum == -1 { -value } else { value }),
                    None => {
                        emit.emit(Rich::custom(
                            extra.span(),
                            format!("invalid base 10 integer '{}'", s),
                        ));
                        Token::Number(BigInt::from(0))
                    }
                }
            });

    choice((
        float_with_fraction,
        float_with_exponent,
        hex_int,
        octal_int,
        binary_int,
        decimal_int,
    ))
    .labelled("numeral")
}

fn identifier_parser<'src>()
-> impl Parser<'src, &'src str, Token<'src>, extra::Err<Rich<'src, char>>> + Clone {
    let meta_identifier = just("!")
        .or_not()
        .then(
            any()
                .filter(|c: &char| c.is_ident_continue())
                .repeated()
                .at_least(1)
        )
        .to_slice()
        .then(
            just(".")
                .ignore_then(
                    any()
                        .filter(|c: &char| c.is_ident_continue())
                        .repeated()
                        .to_slice()
                )
                .repeated()
                .collect::<Vec<_>>()
        )
        .validate(|(s, other): (&str, Vec<&str>), extra, emit| {
            let is_meta = s.starts_with('!');

            if !is_meta && other.is_empty() {
                match s {
                    "void" => return Token::Void,
                    "import" => return Token::Import,
                    _ => {}
                }
            }

            if !is_meta && let Ok(visibility) = Visibility::from_str(s) {
                if !other.is_empty() {
                    emit.emit(Rich::custom(
                        extra.span(),
                        format!(
                            "visibility '{}' does not take any variants, but variants were provided",
                            s
                        ),
                    ));
                }
                return Token::Visibility(visibility);
            }

            if !is_meta && let Ok(cc) = CallingConvention::from_str(s) {
                if !other.is_empty() {
                    emit.emit(Rich::custom(
                        extra.span(),
                        format!(
                            "calling convention '{}' does not take any variants, but variants were provided",
                            s
                        ),
                    ));
                }
                return Token::CallingConvention(cc);
            }

            if let Ok(instr_op) = HyInstrOp::from_str(s) {
                return Token::InstrOp(instr_op, other);
            }

            if let Ok(terminator_op) = HyTerminatorOp::from_str(s) {
                if !other.is_empty() {
                    emit.emit(Rich::custom(
                        extra.span(),
                        format!(
                            "terminator operation '{}' does not take any variants, but variants were provided",
                            s
                        ),
                    ));
                }

                return Token::TerminatorOp(terminator_op);
            }

            if other.is_empty() && !is_meta {
                if let Ok(ftype) = FType::from_str(s) {
                    return Token::FType(ftype);
                }

                if let Ok(ordering) = MemoryOrdering::from_str(s) {
                    return Token::Ordering(ordering);
                }

                if s.starts_with("i") && s[1..].seq_iter().all(|x| x.is_ascii_digit()) {
                    let width_str = &s[1..];
                    match width_str.parse::<u32>() {
                        Ok(width) => {
                            if let Some(itype) = IType::try_new(width) {
                                return Token::IType(itype);
                            } else {
                                emit.emit(Rich::custom(
                                    extra.span(),
                                    format!(
                                        "cannot create IType with width {} (must be between {} and {})",
                                        width,
                                        IType::MIN_BITS,
                                        IType::MAX_BITS
                                    ),
                                ));
                            }
                        }
                        Err(e) => {
                            emit.emit(Rich::custom(
                                extra.span(),
                                format!("invalid integer type width '{}': {}", width_str, e),
                            ));
                        }
                    }
                }
            }

            if is_meta {
                Token::MetaIdentifier(&s[1..], other)
            } else {
                Token::Identifier(s, other)
            }
        });

    fast_boxed!(meta_identifier.labelled("identifier or keyword or itype"))
}

fn register_parser<'src>()
-> impl Parser<'src, &'src str, &'src str, extra::Err<Rich<'src, char>>> + Clone {
    just("%")
        .ignore_then(
            any()
                .filter(|c: &char| c.is_ident_continue() || *c == '.')
                .repeated()
                .to_slice(),
        )
        .labelled("register")
}

fn comment_parser<'src>() -> impl Parser<'src, &'src str, (), extra::Err<Rich<'src, char>>> + Clone
{
    just(";")
        .ignore_then(any().filter(|&c: &char| c != '\n').repeated())
        .ignored()
        .labelled("comment")
}

fn ignoring_parser<'src>() -> impl Parser<'src, &'src str, (), extra::Err<Rich<'src, char>>> + Clone
{
    choice((
        any()
            .filter(|c: &char| c.is_whitespace() && *c != '\n')
            .to(()),
        comment_parser(),
    ))
    .repeated()
    .labelled("whitespace or comment")
}

fn lexer<'src>()
-> impl Parser<'src, &'src str, Vec<Spanned<Token<'src>>>, extra::Err<Rich<'src, char>>> {
    choice((
        numeral_parser(),
        string_parser().map(Token::StringLiteral),
        just("(").to(Token::LParen),
        just(")").to(Token::RParen),
        just("{").to(Token::LBrace),
        just("}").to(Token::RBrace),
        just("[").to(Token::LBracket),
        just("]").to(Token::RBracket),
        just("<").to(Token::LAngle),
        just(">").to(Token::RAngle),
        just(",").to(Token::Comma),
        just(":").to(Token::Colon),
        just("=").to(Token::Equals),
        just("\n")
            .padded_by(ignoring_parser())
            .repeated()
            .at_least(1)
            .to(Token::Newline),
        register_parser().map(Token::Register),
        identifier_parser(),
    ))
    .padded_by(ignoring_parser())
    .map_with(|item, extra| (item, extra.span()))
    .repeated()
    .collect::<Vec<_>>()
}

type Extra<'src> = extra::Full<Rich<'src, Token<'src>, Span>, SimpleState<State<'src>>, ()>;

fn primary_basic_type_parser<'src, I>()
-> impl Parser<'src, I, PrimaryBasicType, Extra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span> + Clone,
{
    any()
        .filter(|x: &Token| {
            x.is_i_type()
                || x.is_f_type()
                || x.try_as_identifier_ref()
                    .map(|x| x.1.is_empty() && *x.0 == "ptr")
                    .unwrap_or(false)
        })
        .map(|token| match token {
            Token::IType(itype) => PrimaryBasicType::Int(itype),
            Token::FType(ftype) => PrimaryBasicType::Float(ftype),
            Token::Identifier(s, v) if s == "ptr" && v.is_empty() => PrimaryBasicType::Ptr(PtrType),
            _ => unreachable!(),
        })
}

fn primary_type_parser<'src, I>() -> impl Parser<'src, I, PrimaryType, Extra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span> + Clone,
{
    let primary_type = primary_basic_type_parser().map(|prim_type| prim_type.into());

    // Vector types (e.g., <4 x i32> or <vscale 4 x i32>)
    let vector_type = just(Token::Identifier("vscale", vec![]))
        .or_not()
        .then(
            just_match(TokenDiscriminants::Number).validate(|num_span, extra, emit| {
                let num = num_span.try_as_number().unwrap();

                if num <= BigInt::ZERO {
                    emit.emit(Rich::custom(
                        extra.span(),
                        "vector size must be a positive non-zero integer",
                    ));
                    1u16
                } else if num > BigInt::from(u16::MAX) {
                    emit.emit(Rich::custom(
                        extra.span(),
                        format!(
                            "vector size too large: maximum allowed is {}, got {}",
                            u16::MAX,
                            num
                        ),
                    ));
                    1u16
                } else {
                    num.to_u32_digits().1.into_iter().next().unwrap() as u16
                }
            }),
            // .map(|(num_token, num_span)| num_token.try_as_number().unwrap()),
        )
        .then_ignore(just(Token::Identifier("x", vec![])))
        .then(primary_basic_type_parser())
        .delimited_by(just(Token::LAngle), just(Token::RAngle))
        .map(|((is_vscale, num), ty)| {
            PrimaryType::Vc(VcType {
                ty,
                size: if is_vscale.is_some() {
                    VcSize::Scalable(num)
                } else {
                    VcSize::Fixed(num)
                },
            })
        });

    choice((primary_type, vector_type)).labelled("primary type")
}

fn type_parser<'src, I>() -> impl Parser<'src, I, Typeref, Extra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span> + Clone,
{
    fast_boxed!(
        recursive(|tree| {
            // Primary basic types
            let primary_type = primary_type_parser().map_with(move |prim_type, extra| {
                extra
                    .state()
                    .type_registry
                    .search_or_insert(prim_type.into())
            });

            // Array types (e.g., [10 x i32])
            let array_type = just(Token::LBracket)
                .ignore_then(just_match(TokenDiscriminants::Number))
                .then_ignore(just(Token::Identifier("x", vec![])))
                .then(tree.clone())
                .then_ignore(just(Token::RBracket))
                .validate(|(size_token, ty), extra, emit| {
                    let size_token = size_token.try_as_number().unwrap();
                    let num_elements = if size_token <= BigInt::ZERO {
                        emit.emit(Rich::custom(
                            extra.span(),
                            "array size must be a positive non-zero integer",
                        ));
                        1u16
                    } else if size_token > BigInt::from(u16::MAX) {
                        emit.emit(Rich::custom(
                            extra.span(),
                            format!(
                                "array size too large: maximum allowed is {}, got {}",
                                u16::MAX,
                                size_token
                            ),
                        ));
                        1u16
                    } else {
                        size_token.to_u32_digits().1.into_iter().next().unwrap() as u16
                    };
                    let array_type = ArrayType { ty, num_elements };
                    let state: &mut SimpleState<State<'src>> = extra.state();
                    state.type_registry.search_or_insert(array_type.into())
                })
                .labelled("array type");

            // Structure types (e.g., { i32, fp32, [4 x i8] })
            let struct_type = just(Token::Identifier("packed", vec![]))
                .or_not()
                .then(
                    tree.clone()
                        .separated_by(just(Token::Comma))
                        .collect::<Vec<_>>()
                        .delimited_by(just(Token::LBrace), just(Token::RBrace)),
                )
                .map_with(|(packed, element_types), extra| {
                    let struct_type = StructType {
                        element_types,
                        packed: packed.is_some(),
                    };
                    let state: &mut SimpleState<State<'src>> = extra.state();
                    state.type_registry.search_or_insert(struct_type.into())
                })
                .labelled("struct type");

            choice((primary_type, array_type, struct_type))
        })
        .labelled("type")
    )
}

fn constant_parser<'src, I>() -> impl Parser<'src, I, AnyConst, Extra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span> + Clone,
{
    let itype_const = just_match(TokenDiscriminants::IType)
        .then(just_match(TokenDiscriminants::Number))
        .map(|(a, b)| {
            AnyConst::Int(IConst {
                ty: a.try_as_i_type().unwrap(),
                value: b.try_as_number().unwrap(),
            })
        })
        .labelled("integer constant");

    let ftype_const = just_match(TokenDiscriminants::FType)
        .then(just_match(TokenDiscriminants::Decimal))
        .map(|(a, b)| {
            AnyConst::Float(FConst {
                ty: a.try_as_f_type().unwrap(),
                value: b.try_as_decimal().unwrap(),
            })
        })
        .labelled("floating-point constant");

    let func_ptr = just(Token::Identifier("ptr", vec![]))
        .ignore_then(just(Token::Identifier("external", vec![])).to(()).or_not())
        .then(
            just_match(TokenDiscriminants::Identifier)
                .map(|token| token.try_as_identifier().unwrap()),
        )
        .validate(move |(external, name), extra, emit| {
            let name = {
                let mut full_name = name.0.to_string();
                for part in name.1 {
                    full_name.push('.');
                    full_name.push_str(part);
                }
                full_name
            };
            let ftype = if external.is_some() {
                FunctionPointerType::External
            } else {
                FunctionPointerType::Internal
            };

            let state: &mut SimpleState<State<'src>> = extra.state();
            state.func_retriever.as_ref()(name.clone(), ftype);
            match (state.func_retriever.as_ref())(name.clone(), ftype) {
                Some(uuid) => match ftype {
                    FunctionPointerType::Internal => {
                        AnyConst::FuncPtr(FunctionPointer::Internal(uuid))
                    }
                    FunctionPointerType::External => {
                        AnyConst::FuncPtr(FunctionPointer::External(uuid))
                    }
                },

                None => {
                    emit.emit(Rich::custom(
                        extra.span(),
                        format!(
                            "{}function pointer '{}' not found",
                            if external.is_some() { "external " } else { "" },
                            name
                        ),
                    ));

                    AnyConst::FuncPtr(FunctionPointer::Internal(Uuid::nil()))
                }
            }
        })
        .labelled("function pointer");

    fast_boxed!(choice((itype_const, ftype_const, func_ptr)))
}

fn label_parser<'src, I>() -> impl Parser<'src, I, Label, Extra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span> + Clone,
{
    just_match(TokenDiscriminants::Identifier)
        .map_with(move |token, extra| {
            let ident = token.try_as_identifier().unwrap();
            let mut full_name = ident.0.to_string();
            for part in ident.1 {
                full_name.push('.');
                full_name.push_str(part);
            }

            let state: &mut SimpleState<State<'src>> = extra.state();
            state.get_label(&full_name)
        })
        .labelled("label")
}

fn register_parser_a<'src, I>() -> impl Parser<'src, I, Name, Extra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span> + Clone,
{
    just_match(TokenDiscriminants::Register)
        .map_with(move |token, extra| {
            let state: &mut SimpleState<State<'src>> = extra.state();
            state.get_register(token.try_as_register().unwrap())
        })
        .labelled("register")
}

fn operand_parser<'src, I>() -> impl Parser<'src, I, Operand, Extra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span> + Clone,
{
    let reg_parser = register_parser_a().map(Operand::Reg);
    let undef_parser = type_parser()
        .then_ignore(just(Token::Identifier("undef", vec![])))
        .map(Operand::Undef)
        .labelled("undef operand");
    let const_parser = constant_parser()
        .map(Operand::Imm)
        .labelled("immediate operand");

    fast_boxed!(choice((reg_parser, undef_parser, const_parser)))
}

fn parse_instruction<'src, I>() -> impl Parser<'src, I, HyInstr, Extra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span> + Clone,
{
    let operand_parser = fast_boxed!(choice((
        /* Use by phi instructions */
        operand_parser()
            .then_ignore(just(Token::Comma))
            .then(label_parser())
            .delimited_by(just(Token::LBracket), just(Token::RBracket))
            .separated_by(just(Token::Comma))
            .at_least(1)
            .collect::<Vec<_>>()
            .map(Either::Right)
            .labelled("phi operand list"),
        /* Use by most instructions */
        operand_parser()
            .separated_by(just(Token::Comma))
            .collect::<Vec<_>>()
            .map(Either::Left)
            .labelled("operand list"),
    )));

    just_match(TokenDiscriminants::Register)
        .map(|x| x.try_as_register().unwrap())
        .then_ignore(just(Token::Colon))
        .then(type_parser())
        .then_ignore(just(Token::Equals))
        .or_not()
        .then(
            just_match(TokenDiscriminants::InstrOp)
                .map(|x| x.try_as_instr_op().unwrap()),
        )
        .then(
            type_parser().then_ignore(just(Token::Comma)).or_not()
        )
        .then(operand_parser)
        .then(
            label_parser()
                .separated_by(just(Token::Comma))
                .collect::<Vec<_>>()
                .or_not(),
        )
        .then(
            just(Token::Comma)
            .ignore_then(
                just(Token::Identifier("align", vec![])),
            )
            .ignore_then(just_match(TokenDiscriminants::Number))
            .validate(|num_token, extra, emit| {
                let align = num_token.try_as_number().unwrap();
                if align <= BigInt::from(0) || align > BigInt::from(u32::MAX) {
                    emit.emit(Rich::custom(
                       extra.span(),
                        format!(
                            "invalid alignment value: must be between 1 and {}, got {}",
                            u32::MAX,
                            align
                        ),
                    ));
                }

                align.to_u32_digits().1.into_iter().next().unwrap_or_default()
            })
            .or_not(),
        )
        .then(
            just(Token::Comma)
                .then(
                    just(Token::Identifier("volatile", vec![]))
                ).to(())
                .or_not(),
        )
        .validate(move |(((elem, labels), align), volatile), extra, emit| {
            let state: &mut SimpleState<State<'src>> = extra.state();
            let (((destination, op), op_additional_ty), operand) = elem;
            let (op, variant) = op;
            let dest_and_ty = if let Some((dest, ty)) = destination {
                Some((state.get_register(dest), ty))
            } else {
                None
            };

            if op_additional_ty.is_some() != matches!(op, HyInstrOp::MGetElementPtr) {
                if op_additional_ty.is_some() {
                    emit.emit(Rich::custom(
                        extra.span(),
                        format!(
                            "syntax error for {} instruction: unexpected additional type",
                            op.opname()
                        ),
                    ));
                } else {
                    emit.emit(Rich::custom(
                        extra.span(),
                        format!(
                            "syntax error for {} instruction: missing additional type",
                            op.opname()
                        ),
                    ));
                }

                return HyInstr::MetaAssert(MetaAssert { condition: Operand::Imm(IConst::from(1u64).into()) });
            }

            if op != HyInstrOp::Phi && matches!(operand, Either::Right(_)) {
                emit.emit(Rich::custom(
                    extra.span(),
                    format!(
                        "syntax error for {} instruction: only 'phi' instructions can use the [operand, label] syntax, use operands separated by commas instead",
                        op.opname()
                    )
                ));

                return HyInstr::MetaAssert(MetaAssert { condition: Operand::Imm(IConst::from(1u64).into()) });
            }
            else if op == HyInstrOp::Phi && matches!(operand, Either::Left(_)) {
                emit.emit(Rich::custom(
                    extra.span(),
                    "syntax error for phi instruction: expected [operand, label] pairs, got operands separated by commas instead".to_string(),
                ));

                return HyInstr::MetaAssert(MetaAssert { condition: Operand::Imm(IConst::from(1u64).into()) });
            }

            if matches!(op, HyInstrOp::MStore | HyInstrOp::MetaAssert | HyInstrOp::MetaAssume) {
                if dest_and_ty.is_some() {
                    emit.emit(Rich::custom(
                        extra.span(),
                        format!(
                            "syntax error for {} instruction: destination register specified where none expected",
                            op.opname()
                        ),
                    ));

                    return HyInstr::MetaAssert(MetaAssert { condition: Operand::Imm(IConst::from(1u64).into()) });
                }
            }
            else if op != HyInstrOp::Invoke && dest_and_ty.is_none() {
                emit.emit(Rich::custom(
                    extra.span(),
                    format!(
                        "syntax error for {} instruction: missing destination register",
                        op.opname()
                    ),
                ));

                return HyInstr::MetaAssert(MetaAssert { condition: Operand::Imm(IConst::from(1u64).into()) });
            }

            // Only authorize label lists for !analysis.term.reach (future-proof: parsed for all)
            if let Some(lbls) = labels.as_ref() {
                let first_variant = variant.first().copied();
                let second_variant = variant.get(1).copied();

                if !(lbls.is_empty() || op == HyInstrOp::MetaAnalysisStat && first_variant == Some("term") && second_variant == Some("reach")) {
                    emit.emit(Rich::custom(
                        extra.span(),
                        format!(
                            "syntax error for {} instruction: unexpected label list",
                            op.opname()
                        ),
                    ));

                    return HyInstr::MetaAssert(MetaAssert { condition: Operand::Imm(IConst::from(1u64).into()) });
                }
            }

            if matches!(op, HyInstrOp::MLoad | HyInstrOp::MStore) {
                /* Load and store instructions can both have one or zero variant operands */
                if variant.len() > 1 {
                    emit.emit(Rich::custom(
                        extra.span(),
                        format!(
                            "arity mismatch for {} instruction variant: expected at most 1 variant operand, got {}",
                            op.opname(),
                            variant.len()
                        ),
                    ));

                    return HyInstr::MetaAssert(MetaAssert { condition: Operand::Imm(IConst::from(1u64).into()) });
                }
            }
            else if op.has_variant() == variant.is_empty() {
                emit.emit(Rich::custom(
                    extra.span(),
                    format!(
                        "syntax error for {} instruction: expected {}, got {}",
                        op.opname(),
                        if op.has_variant() { "variant" } else { "no variant" },
                        if variant.is_empty() { "no variant" } else { "variant" }
                    ),
                ));

                return HyInstr::MetaAssert(MetaAssert { condition: Operand::Imm(IConst::from(1u64).into()) });
            }

            if op.has_variant() {
                let num_variant_operands = match op {
                    HyInstrOp::MetaAnalysisStat => {
                        if variant.is_empty() {
                            1
                        } else {
                            match AnalysisStatisticOp::from_str(variant[0]) {
                                Ok(AnalysisStatisticOp::ExecutionCount) => 1,
                                Ok(AnalysisStatisticOp::InstructionCount) => 1,
                                Ok(AnalysisStatisticOp::TerminationBehavior) => 2,
                                Err(()) => 1,
                            }
                        }
                    }
                    _ => 1,
                };

                if variant.len() != num_variant_operands {
                    emit.emit(Rich::custom(
                        extra.span(),
                        format!(
                            "arity mismatch for {} instruction variant: expected {} variant operands, got {}",
                            op.opname(),
                            num_variant_operands,
                            variant.len()
                        ),
                    ));

                    return HyInstr::MetaAssert(MetaAssert { condition: Operand::Imm(IConst::from(1u64).into()) });
                }
            }

            if let Some(arity) = op.arity() {
                // only phi-instructions can have right variant operands, therefore we 
                // asume left here
                let operand = operand.as_ref().unwrap_left();
                if operand.len() != arity {

                    emit.emit(Rich::custom(
                        extra.span(),
                        format!(
                            "arity mismatch for {} instruction: expected {} operands, got {}",
                            op.opname(),
                            arity,
                            operand.len()
                        ),
                    ));

                    return HyInstr::MetaAssert(MetaAssert { condition: Operand::Imm(IConst::from(1u64).into()) });
                }
            }

            if align.is_some() && !matches!(op, HyInstrOp::MLoad | HyInstrOp::MStore | HyInstrOp::MAlloca) {
                emit.emit(Rich::custom(
                    extra.span(),
                    format!(
                        "alignment specifier is only valid for load, store and alloca instructions, got {} instruction",
                        op.opname()
                    ),
                ));
            }

            if volatile.is_some() && !matches!(op, HyInstrOp::MLoad | HyInstrOp::MStore) {
                emit.emit(Rich::custom(
                    extra.span(),
                    format!(
                        "volatile specifier is only valid for load and store instructions, got {} instruction",
                        op.opname()
                    ),
                ));
            }

            match op {
                HyInstrOp::IAdd | HyInstrOp::ISub | HyInstrOp::IMul => {
                    let [lhs, rhs] = operand.unwrap_left().try_into().unwrap();
                    let (dest, ty) = dest_and_ty.unwrap();
                    let variant = match OverflowSignednessPolicy::from_str(variant[0]) {
                        Ok(variant) => variant,
                        Err(()) => {
                            emit.emit(Rich::custom(
                                extra.span(),
                                format!(
                                    "unknown overflow signedness policy: {} (expected one of: {})",
                                    variant[0],
                                    OverflowSignednessPolicy::iter()
                                        .map(|x| x.to_str())
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                ),
                            ));
                            OverflowSignednessPolicy::Wrap
                        }
                    };

                    match op {
                        HyInstrOp::IAdd => IAdd { dest, ty, lhs, rhs, variant }.into(),
                        HyInstrOp::ISub => ISub { dest, ty, lhs, rhs, variant }.into(),
                        HyInstrOp::IMul => IMul { dest, ty, lhs, rhs, variant }.into(),
                        _ => unreachable!(),
                    }
                }
                HyInstrOp::IDiv | HyInstrOp::IRem => {
                    let [lhs, rhs] = operand.unwrap_left().try_into().unwrap();
                    let (dest, ty) = dest_and_ty.unwrap();
                    let signedness = match IntegerSignedness::from_str(variant[0]) {
                        Ok(variant) => variant,
                        Err(()) => {
                            emit.emit(Rich::custom(
                                extra.span(),
                                format!(
                                    "unknown signedness variant: {} (expected one of: {})",
                                    variant[0],
                                    IntegerSignedness::iter()
                                        .map(|x| x.to_str())
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                ),
                            ));
                            IntegerSignedness::Unsigned
                        }
                    };

                    match op {
                        HyInstrOp::IDiv => IDiv { dest, ty, lhs, rhs, signedness }.into(),
                        HyInstrOp::IRem => IRem { dest, ty, lhs, rhs, signedness }.into(),
                        _ => unreachable!(),
                    }
                }
                HyInstrOp::ISht => {
                    let [lhs, rhs] = operand.unwrap_left().try_into().unwrap();
                    let (dest, ty) = dest_and_ty.unwrap();
                    let variant = match IShiftVariant::from_str(variant[0]) {
                        Ok(variant) => variant,
                        Err(()) => {
                            emit.emit(Rich::custom(
                                extra.span(),
                                format!(
                                    "unknown integer isht variant: {} (expected one of: {})",
                                    variant[0],
                                    IShiftVariant::iter()
                                        .map(|x| x.to_str())
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                ),
                            ));
                            IShiftVariant::Asr
                        }
                    };

                    ISht { dest, ty, lhs, rhs, variant }.into()
                }
                HyInstrOp::FNeg => {
                    let [value] = operand.unwrap_left().try_into().unwrap();
                    let (dest, ty) = dest_and_ty.unwrap();

                    FNeg { dest, ty, value }.into()
                }
                HyInstrOp::INeg => {
                    let [value] = operand.unwrap_left().try_into().unwrap();
                    let (dest, ty) = dest_and_ty.unwrap();

                    INeg { dest, ty, value }.into()
                }
                HyInstrOp::INot => {
                    let [value] = operand.unwrap_left().try_into().unwrap();
                    let (dest, ty) = dest_and_ty.unwrap();

                    INot { dest, ty, value }.into()
                }
                HyInstrOp::IAnd |
                HyInstrOp::IOr |
                HyInstrOp::IXor |
                HyInstrOp::IImplies |
                HyInstrOp::IEquiv |
                HyInstrOp::FAdd |
                HyInstrOp::FSub |
                HyInstrOp::FMul |
                HyInstrOp::FDiv |
                HyInstrOp::FRem => {
                    let [lhs, rhs] = operand.unwrap_left().try_into().unwrap();
                    let (dest, ty) = dest_and_ty.unwrap();

                    match op {
                        HyInstrOp::IAnd => IAnd { dest, ty, lhs, rhs }.into(),
                        HyInstrOp::IOr => IOr { dest, ty, lhs, rhs }.into(),
                        HyInstrOp::IXor => IXor { dest, ty, lhs, rhs }.into(),
                        HyInstrOp::INot => INot { dest, ty, value: lhs }.into(),
                        HyInstrOp::IImplies => IImplies { dest, ty, lhs, rhs }.into(),
                        HyInstrOp::IEquiv => IEquiv { dest, ty, lhs, rhs }.into(),
                        HyInstrOp::FAdd => FAdd { dest, ty, lhs, rhs }.into(),
                        HyInstrOp::FSub => FSub { dest, ty, lhs, rhs }.into(),
                        HyInstrOp::FMul => FMul { dest, ty, lhs, rhs }.into(),
                        HyInstrOp::FDiv => FDiv { dest, ty, lhs, rhs }.into(),
                        HyInstrOp::FRem => FRem { dest, ty, lhs, rhs }.into(),
                        _ => unreachable!(),
                    }
                }
                HyInstrOp::FCmp => {
                    let [lhs, rhs] = operand.unwrap_left().try_into().unwrap();
                    let (dest, ty) = dest_and_ty.unwrap();
                    let variant = match FCmpVariant::from_str(variant[0]) {
                        Ok(variant) => variant,
                        Err(()) => {
                            emit.emit(Rich::custom(
                                extra.span(),
                                format!(
                                    "unknown floating-point comparison variant: {} (expected one of: {})",
                                    variant[0],
                                    FCmpVariant::iter()
                                        .map(|x| x.to_str())
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                ),
                            ));
                            FCmpVariant::One
                        }
                    };

                    FCmp { dest, ty, lhs, rhs, variant }.into()
                }
                HyInstrOp::ICmp => {
                    let [lhs, rhs] = operand.unwrap_left().try_into().unwrap();
                    let (dest, ty) = dest_and_ty.unwrap();

                    let variant = match ICmpVariant::from_str(variant[0]) {
                        Ok(variant) => variant,
                        Err(()) => {
                            emit.emit(Rich::custom(
                                extra.span(),
                                format!(
                                    "unknown integer comparison variant: {} (expected one of: {})",
                                    variant[0],
                                    ICmpVariant::iter()
                                        .map(|x| x.to_str())
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                ),
                            ));
                            ICmpVariant::Eq
                        }
                    };

                    ICmp { dest, ty, lhs, rhs, variant }.into()
                },
                HyInstrOp::MLoad => {
                    let [addr] = operand.unwrap_left().try_into().unwrap();
                    let (dest, ty) = dest_and_ty.unwrap();
                    let ordering = if variant.is_empty() {
                        None
                    } else {
                        if variant.len() != 1 {
                            emit.emit(Rich::custom(
                                extra.span(),
                                format!(
                                    "arity mismatch for {} instruction variant: expected 1 variant operand, got {}",
                                    op.opname(),
                                    variant.len()
                                ),
                            ));

                            return HyInstr::MetaAssert(MetaAssert { condition: Operand::Imm(IConst::from(1u64).into()) });
                        }

                        Some(
                            match MemoryOrdering::from_str(variant[0]) {
                                Ok(op) => op,
                                Err(()) => {
                                    emit.emit(Rich::custom(
                                        extra.span(),
                                        format!(
                                            "unknown memory ordering variant: {} (expected one of: {})",
                                            variant[0],
                                            MemoryOrdering::iter()
                                                .map(|x| x.to_str())
                                                .collect::<Vec<_>>()
                                                .join(", ")
                                        ),
                                    ));
                                    MemoryOrdering::AcqRel
                                }
                            }
                        )
                    };

                    MLoad { dest, ty, addr, alignement: align, ordering, volatile: volatile.is_some() }.into()
                }
                HyInstrOp::MStore => {
                    let [addr, value] = operand.unwrap_left().try_into().unwrap();
                    let ordering = if variant.is_empty() {
                        None
                    } else {
                        Some(
                            match MemoryOrdering::from_str(variant[0]) {
                                Ok(op) => op,
                                Err(()) => {
                                    emit.emit(Rich::custom(
                                        extra.span(),
                                        format!(
                                            "unknown memory ordering variant: {} (expected one of: {})",
                                            variant[0],
                                            MemoryOrdering::iter()
                                                .map(|x| x.to_str())
                                                .collect::<Vec<_>>()
                                                .join(", ")
                                        ),
                                    ));
                                    MemoryOrdering::AcqRel
                                }
                            }
                        )
                    };

                    MStore { addr, value, alignement: align, ordering, volatile: volatile.is_some() }.into()
                },
                HyInstrOp::MAlloca => {
                    let [count] = operand.unwrap_left().try_into().unwrap();
                    let (dest, ty) = dest_and_ty.unwrap();

                    MAlloca { dest, ty, count, alignement: align }.into()
                }
                HyInstrOp::MGetElementPtr => {
                    let mut indices = operand.unwrap_left();
                    let (dest, ty) = dest_and_ty.unwrap();
                    let op_additional_ty = op_additional_ty.unwrap();

                    if indices.is_empty() {
                        emit.emit(Rich::custom(
                            extra.span(),
                            format!(
                                "arity mismatch for {} instruction: expected at least 1 operand for indices, got 0",
                                op.opname(),
                            ),
                        ));

                        return HyInstr::MetaAssert(MetaAssert { condition: Operand::Imm(IConst::from(1u64).into()) });
                    }

                    let base = indices.remove(0);

                    MGetElementPtr { dest, ty, in_ty: op_additional_ty, base, indices }.into()
                }
                HyInstrOp::Invoke => {
                    let mut operands = operand.unwrap_left();
                    let (dest, ty) = match dest_and_ty {
                        Some((dest, ty)) => (Some(dest), Some(ty)),
                        None => (None, None),
                    };

                    if operands.is_empty() {
                        emit.emit(Rich::custom(
                            extra.span(),
                            format!(
                                "arity mismatch for {} instruction: expected at least 1 operand (function pointer), got {}",
                                op.opname(),
                                operands.len(),
                            ),
                        ));

                        return HyInstr::MetaAssert(MetaAssert { condition: Operand::Imm(IConst::from(1u64).into()) });
                    }

                    let function = operands.remove(0);

                    Invoke { dest, ty, function, args: operands, cconv: None }.into()
                },
                HyInstrOp::Phi => {
                    let (dest, ty) = dest_and_ty.unwrap();
                    Phi { dest, ty, values: operand.unwrap_right() }.into()
                }
                HyInstrOp::Select => {
                    let [condition, true_value, false_value] = operand.unwrap_left().try_into().unwrap();
                    let (dest, ty) = dest_and_ty.unwrap();

                    Select { dest, ty, condition, true_value, false_value }.into()
                }
                HyInstrOp::Cast => {
                    let [value] = operand.unwrap_left().try_into().unwrap();
                    let (dest, ty) = dest_and_ty.unwrap();
                    let variant = match CastVariant::from_str(variant[0]) {
                        Ok(op) => op,
                        Err(()) => {
                            emit.emit(Rich::custom(
                                extra.span(),
                                format!(
                                    "unknown cast variant: {} (expected one of: {})",
                                    variant[0],
                                    CastVariant::iter()
                                        .map(|x| x.to_str())
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                ),
                            ));
                            CastVariant::Trunc
                        }
                    };

                    Cast { dest, ty, value, variant }.into()
                }
                HyInstrOp::InsertValue => {
                    let mut operands = operand.unwrap_left();
                    let (dest, ty) = dest_and_ty.unwrap();

                    if operands.len() < 3 {
                        emit.emit(Rich::custom(
                            extra.span(),
                            format!(
                                "arity mismatch for {} instruction: expected aggregate, value, and at least one index, got {}",
                                op.opname(),
                                operands.len()
                            ),
                        ));

                        return HyInstr::MetaAssert(MetaAssert { condition: Operand::Imm(IConst::from(1u64).into()) });
                    }

                    let aggregate = operands.remove(0);
                    let value = operands.remove(0);

                    let mut indices = Vec::with_capacity(operands.len());
                    for (idx, op_idx) in operands.into_iter().enumerate() {
                        if let Operand::Imm(AnyConst::Int(iconst)) = op_idx {
                            let (sign, digits) = iconst.value.to_u32_digits();
                            if sign == Sign::Minus {
                                emit.emit(Rich::custom(
                                    extra.span(),
                                    format!("index {} must be non-negative", idx),
                                ));
                                continue;
                            }
                            if digits.len() > 1 {
                                emit.emit(Rich::custom(
                                    extra.span(),
                                    format!("index {} is too large to fit in u32", idx),
                                ));
                                continue;
                            }
                            indices.push(digits.first().cloned().unwrap_or(0));
                        } else {
                            emit.emit(Rich::custom(
                                extra.span(),
                                format!("index {} must be an integer immediate", idx),
                            ));
                        }
                    }

                    InsertValue { dest, ty, aggregate, value, indices }.into()
                }
                HyInstrOp::ExtractValue => {
                    let mut operands = operand.unwrap_left();
                    let (dest, ty) = dest_and_ty.unwrap();

                    if operands.len() < 2 {
                        emit.emit(Rich::custom(
                            extra.span(),
                            format!(
                                "arity mismatch for {} instruction: expected aggregate and at least one index, got {}",
                                op.opname(),
                                operands.len()
                            ),
                        ));

                        return HyInstr::MetaAssert(MetaAssert { condition: Operand::Imm(IConst::from(1u64).into()) });
                    }

                    let aggregate = operands.remove(0);

                    let mut indices = Vec::with_capacity(operands.len());
                    for (idx, op_idx) in operands.into_iter().enumerate() {
                        if let Operand::Imm(AnyConst::Int(iconst)) = op_idx {
                            let (sign, digits) = iconst.value.to_u32_digits();
                            if sign == Sign::Minus {
                                emit.emit(Rich::custom(
                                    extra.span(),
                                    format!("index {} must be non-negative", idx),
                                ));
                                continue;
                            }
                            if digits.len() > 1 {
                                emit.emit(Rich::custom(
                                    extra.span(),
                                    format!("index {} is too large to fit in u32", idx),
                                ));
                                continue;
                            }
                            indices.push(digits.first().cloned().unwrap_or(0));
                        } else {
                            emit.emit(Rich::custom(
                                extra.span(),
                                format!("index {} must be an integer immediate", idx),
                            ));
                        }
                    }

                    ExtractValue { dest, ty, aggregate, indices }.into()
                }
                HyInstrOp::MetaAssert => {
                    let [condition] = operand.unwrap_left().try_into().unwrap();

                    MetaAssert { condition }.into()
                }
                HyInstrOp::MetaAssume => {
                    let [condition] = operand.unwrap_left().try_into().unwrap();

                    MetaAssume { condition }.into()
                }
                HyInstrOp::MetaIsDef => {
                    let [value] = operand.unwrap_left().try_into().unwrap();
                    let (dest, ty) = dest_and_ty.unwrap();

                    MetaIsDef { dest, ty, operand: value }.into()
                }
                HyInstrOp::MetaProb => {
                    let (dest, ty) = dest_and_ty.unwrap();
                    let variant = match MetaProbVariant::from_str(variant[0]) {
                        Ok(variant) => variant,
                        Err(()) => {
                            emit.emit(Rich::custom(
                                extra.span(),
                                format!(
                                    "unknown meta-probability variant: {} (expected one of: {})",
                                    variant[0],
                                    MetaProbVariant::iter()
                                        .map(|x| x.to_str())
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                ),
                            ));
                            MetaProbVariant::ExpectedValue
                        }
                    };

                    if variant.arity() != operand.as_ref().unwrap_left().len() {
                        emit.emit(Rich::custom(
                            extra.span(),
                            format!(
                                "arity mismatch for meta-probability variant {}: expected {} operands, got {}",
                                variant.to_str(),
                                variant.arity(),
                                operand.as_ref().unwrap_left().len()
                            ),
                        ));

                        return HyInstr::MetaAssert(MetaAssert { condition: Operand::Imm(IConst::from(1u64).into()) });
                    }

                    let operand = match variant {
                        MetaProbVariant::ExpectedValue => {
                            let [value] = operand.unwrap_left().try_into().unwrap();
                            MetaProbOperand::ExpectedValue(value)
                        }
                        MetaProbVariant::Probability => {
                            let [value] = operand.unwrap_left().try_into().unwrap();
                            MetaProbOperand::Probability(value)
                        }
                        MetaProbVariant::Variance => {
                            let [value] = operand.unwrap_left().try_into().unwrap();
                            MetaProbOperand::Variance(value)
                        }
                    };

                    MetaProb { dest, ty, operand }.into()
                }
                HyInstrOp::MetaAnalysisStat => {
                    let (dest, ty) = dest_and_ty.unwrap();

                    if variant.is_empty() {
                        emit.emit(Rich::custom(
                            extra.span(),
                            "missing variant for !analysis",
                        ));
                        return HyInstr::MetaAssert(MetaAssert { condition: Operand::Imm(IConst::from(1u64).into()) });
                    }

                    let stat = match AnalysisStatisticOp::from_str(variant[0]) {
                        Ok(AnalysisStatisticOp::ExecutionCount) => {
                            if let Either::Left(ops) = operand.as_ref()
                                && !ops.is_empty() {
                                emit.emit(Rich::custom(
                                    extra.span(),
                                    format!(
                                        "arity mismatch for !analysis.icnt: expected 0 operands, got {}",
                                        ops.len()
                                    ),
                                ));
                                return HyInstr::MetaAssert(MetaAssert { condition: Operand::Imm(IConst::from(1u64).into()) });
                            }
                            AnalysisStatistic::ExecutionCount
                        }
                        Ok(AnalysisStatisticOp::InstructionCount) => {
                            let ops = match operand.as_ref() {
                                Either::Left(ops) => ops,
                                Either::Right(_) => {
                                    emit.emit(Rich::custom(
                                        extra.span(),
                                        "syntax error for !analysis.icnt: expected 1 immediate integer operand",
                                    ));
                                    return HyInstr::MetaAssert(MetaAssert { condition: Operand::Imm(IConst::from(1u64).into()) });
                                }
                            };
                            if ops.len() != 1 {
                                emit.emit(Rich::custom(
                                    extra.span(),
                                    format!(
                                        "arity mismatch for !analysis.icnt: expected 1 operand, got {}",
                                        ops.len()
                                    ),
                                ));
                                return HyInstr::MetaAssert(MetaAssert { condition: Operand::Imm(IConst::from(1u64).into()) });
                            }
                            let [value] = operand.unwrap_left().try_into().unwrap();
                            match value {
                                Operand::Imm(AnyConst::Int(ic)) => {
                                    let bits = ic.value.to_u32_digits().1.into_iter().next().unwrap_or_default();
                                    let flags = InstructionFlags::from_bits_truncate(bits);
                                    AnalysisStatistic::InstructionCount(flags)
                                }
                                _ => {
                                    emit.emit(Rich::custom(
                                        extra.span(),
                                        "type error for !analysis.icnt: expected integer immediate operand",
                                    ));
                                    return HyInstr::MetaAssert(MetaAssert { condition: Operand::Imm(IConst::from(1u64).into()) });
                                }
                            }
                        }
                        Ok(AnalysisStatisticOp::TerminationBehavior) => {
                            if let Either::Left(ops) = operand.as_ref()
                                && !ops.is_empty() {
                                emit.emit(Rich::custom(
                                    extra.span(),
                                    format!(
                                        "arity mismatch for !analysis.term: expected 0 operands, got {}",
                                        ops.len()
                                    ),
                                ));
                                return HyInstr::MetaAssert(MetaAssert { condition: Operand::Imm(IConst::from(1u64).into()) });
                            }
                            let scope_name = variant.get(1).copied().unwrap_or("");
                            if scope_name == "reach" {
                                let lbls = labels.clone().unwrap_or_default();
                                if lbls.is_empty() {
                                    emit.emit(Rich::custom(
                                        extra.span(),
                                        "syntax error for !analysis.term.reach: expected at least 1 label",
                                    ));
                                    return HyInstr::MetaAssert(MetaAssert { condition: Operand::Imm(IConst::from(1u64).into()) });
                                }
                                AnalysisStatistic::TerminationBehavior(TerminationScope::ReachAny(lbls))
                            } else {
                                let scope = match scope_name {
                                    "blockexit" => TerminationScope::BlockExit,
                                    "funcexit" => TerminationScope::FunctionExit,
                                    _ => {
                                        emit.emit(Rich::custom(
                                            extra.span(),
                                            format!(
                                                "unknown termination scope variant: {} (expected one of: blockexit, funcexit, reach)",
                                                scope_name
                                            ),
                                        ));
                                        return HyInstr::MetaAssert(MetaAssert { condition: Operand::Imm(IConst::from(1u64).into()) });
                                    }
                                };
                                AnalysisStatistic::TerminationBehavior(scope)
                            }
                        }
                        Err(()) => {
                            emit.emit(Rich::custom(
                                extra.span(),
                                format!(
                                    "unknown analysis statistic op: {} (expected one of: {})",
                                    variant[0],
                                    crate::analysis::AnalysisStatisticOp::iter().map(|op| op.to_str()).collect::<Vec<_>>().join(", ")
                                ),
                            ));
                            return HyInstr::MetaAssert(MetaAssert { condition: Operand::Imm(IConst::from(1u64).into()) });
                        }
                    };

                    MetaAnalysisStat { dest, ty, statistic: stat }.into()
                }
                HyInstrOp::MetaForall => {
                    let (dest, ty) = dest_and_ty.unwrap();
                    if let Either::Left(ops) = operand.as_ref()
                        && !ops.is_empty() {
                        emit.emit(Rich::custom(
                            extra.span(),
                            format!(
                                "arity mismatch for !forall: expected 0 operands, got {}",
                                ops.len()
                            ),
                        ));
                        return HyInstr::MetaAssert(MetaAssert { condition: Operand::Imm(IConst::from(1u64).into()) });
                    }
                    MetaForall { dest, ty }.into()
                }
            }
        }).boxed()
}

fn parse_terminator<'src, I>() -> impl Parser<'src, I, HyTerminator, Extra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span> + Clone,
{
    let branch = just(Token::TerminatorOp(HyTerminatorOp::Branch))
        .ignore_then(operand_parser())
        .then_ignore(just(Token::Comma))
        .then(label_parser())
        .then_ignore(just(Token::Comma))
        .then(label_parser())
        .map(|((cond, target_true), target_false)| {
            Branch {
                cond,
                target_true,
                target_false,
            }
            .into()
        });

    let trap = just(Token::TerminatorOp(HyTerminatorOp::Trap)).to(Trap.into());

    let jump = just(Token::TerminatorOp(HyTerminatorOp::Jump))
        .ignore_then(label_parser())
        .map(|target| Jump { target }.into());

    let ret = just(Token::TerminatorOp(HyTerminatorOp::Ret))
        .ignore_then(
            operand_parser()
                .map(Either::Left)
                .or(just(Token::Void).map(Either::Right)),
        )
        .map(|operand| {
            Ret {
                value: operand.left(),
            }
            .into()
        });

    choice((branch, trap, jump, ret)).boxed()
}

fn parse_function<'src, I>() -> impl Parser<'src, I, Function, Extra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span> + Clone,
{
    let block_label = label_parser()
        .then_ignore(just(Token::Colon))
        .then_ignore(just(Token::Newline).or_not());

    let block = block_label
        .then(
            parse_instruction()
                .separated_by(just(Token::Newline))
                .collect::<Vec<_>>(),
        )
        .then_ignore(just(Token::Newline).or_not())
        .then(parse_terminator())
        .then_ignore(just(Token::Newline).or_not())
        .map(|((label, instructions), terminator)| BasicBlock {
            label,
            instructions,
            terminator,
        });

    let meta_arguments = any()
        .filter(|x: &Token| x.is_calling_convention() || x.is_visibility())
        .repeated()
        .at_most(2)
        .collect::<Vec<_>>()
        .validate(|meta_args, extra, emit| {
            let mut seen_cconv = false;
            let mut seen_visibility = false;

            for token in &meta_args {
                if token.is_calling_convention() {
                    if seen_cconv {
                        emit.emit(Rich::custom(
                            extra.span(),
                            "duplicate calling convention metadata",
                        ));
                    }
                    seen_cconv = true;
                } else if token.is_visibility() {
                    if seen_visibility {
                        emit.emit(Rich::custom(extra.span(), "duplicate visibility metadata"));
                    }
                    seen_visibility = true;
                }
            }

            meta_args
        });

    let arglist = register_parser_a()
        .then_ignore(just(Token::Colon))
        .then(type_parser())
        .separated_by(just(Token::Comma))
        .collect::<Vec<_>>()
        .delimited_by(just(Token::LParen), just(Token::RParen));

    fast_boxed!(just(Token::Identifier("define", vec![]))
        .ignore_then(type_parser().map(Either::Left).or(just(Token::Void).map(Either::Right)))
        .then(meta_arguments)
        .then(
            any()
                .filter(|x: &Token| x.is_identifier() || x.is_meta_identifier())
                .map(|x| {
                    let ((full_name, xs), is_meta) = {
                        if x.is_identifier() {
                            (x.try_as_identifier().unwrap(), false)
                        } else {
                            (x.try_as_meta_identifier().unwrap(), true)
                        }
                    };
                    let mut full_name = full_name.to_string();

                    for part in xs {
                        full_name.push('.');
                        full_name.push_str(part);
                    }
                    (full_name, is_meta)
                })
        )
        .then(arglist)
        .then(
            block.repeated()
            .collect::<Vec<_>>()
            .delimited_by(
                just(Token::LBrace).ignore_then(just(Token::Newline).or_not()),
                just(Token::RBrace),
            )
        )
        .map_with(move |((((ty, meta), (func_name, is_meta_func)), params), blocks), extra| {
            let state: &mut SimpleState<State<'src>> = extra.state();
            let uuid = (state.uuid_generator)();
            let mut cconv = None;
            let mut visibility = None;

            for meta_token in meta {
                if meta_token.is_calling_convention() {
                    cconv = Some(meta_token.try_as_calling_convention().unwrap());
                } else if meta_token.is_visibility() {
                    visibility = Some(meta_token.try_as_visibility().unwrap());
                }
            }

            let func = Function {
                uuid,
                name: Some(func_name.to_string()),
                params,
                return_type: ty.left(),
                body: blocks.into_iter().map(|block| (block.label, block)).collect(),
                visibility,
                cconv,
                meta_function: is_meta_func,
                ..Default::default()
            };

            // Check if function should be meta-function
            let should_be_meta = func.should_be_meta_function();
            let is_meta = func.meta_function;

            if should_be_meta && !is_meta {
                error!(
                    "Function '{}' should be declared as a meta-function (it uses meta-instructions or has wildcard types)",
                    func.name.as_ref().unwrap()
                );
            } else if !should_be_meta && is_meta {
                warn!(
                    "Function '{}' is declared as a meta-function but does not use any meta-instructions or wildcard types",
                    func.name.as_ref().unwrap()
                );
            }

            // Clear state namespaces for next function
            state.clear();
            func
        })
        .labelled("function definition"))
}

fn import_parser<'src, I>() -> impl Parser<'src, I, String, Extra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span> + Clone,
{
    // spanned_match_identifie$("import")
    just_match(TokenDiscriminants::Import)
        .ignore_then(
            just_match(TokenDiscriminants::StringLiteral)
                .map(|token| token.try_as_string_literal().unwrap()),
        )
        .labelled("import statement")
}

// Final parser, import + function definitions
enum Item {
    Import(String),
    Function(Function),
}

fn final_parser<'src, I>() -> impl Parser<'src, I, Vec<Item>, Extra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span> + Clone,
{
    fast_boxed!(
        just(Token::Newline)
            .or_not()
            .ignore_then(choice((
                import_parser().map(Item::Import),
                parse_function().map(Item::Function),
            )))
            .then_ignore(just(Token::Newline).or_not())
            .repeated()
            .collect()
    )
}

/// Extend a module by parsing a file at the given path, including handling imports
/// recursively.
///
/// Notes: This function is subject to breaking changes as the parser is developed (notably
/// the import system is yet to be stabilized).
///
/// # Arguments
///  - `module`: The module to extend.
///  - `registry`: The type registry to use for type resolution.
///  - `path`: The path to the source file to parse.
///
/// # Returns
/// - `Ok(())` if the module was successfully extended.
pub fn extend_module_from_path(
    module: &mut Module,
    registry: &TypeRegistry,
    path: impl AsRef<Path>,
) -> Result<(), Error> {
    // Canonicalize the path
    let canonical_path = std::fs::canonicalize(&path)
        .map_err(|e| Error::FileNotFound {
            path: path.as_ref().to_string_lossy().to_string(),
            cause: e,
        })
        .inspect_err(|e| error!("An error occurred while canonicalizing the path: {}", e))?;
    debug!(
        "Extending module from file: {}",
        canonical_path.to_string_lossy()
    );

    // Stack of files to process
    let mut stack = vec![canonical_path];
    let unresolved_internal_functions: RefCell<HashMap<String, Uuid>> = Default::default();
    let unresolved_external_functions: RefCell<HashMap<String, Uuid>> = Default::default();
    let mut list_added_internal_functions = vec![];

    while let Some(current_path) = stack.pop() {
        // Read the source file
        debug!(
            "Reading source file at path: {}",
            current_path.to_string_lossy()
        );
        let source = std::fs::read_to_string(&current_path)
            .map_err(|e| Error::FileNotFound {
                path: current_path.to_string_lossy().to_string(),
                cause: e,
            })
            .inspect_err(|e| error!("An error occurred while reading the source file: {}", e))?;

        // Lex the source file
        let lexer_result = lexer().parse(&source);
        if lexer_result.has_errors() {
            error!(
                "Lexing errors encountered in file {}:",
                current_path.to_string_lossy()
            );

            let errors = lexer_result
                .into_errors()
                .into_iter()
                .map(|e| ParserError {
                    file: Some(current_path.to_string_lossy().to_string()),
                    start: e.span().start,
                    end: e.span().end,
                    message: e.reason().to_string(),
                })
                .collect();
            return Err(Error::ParserErrors {
                errors,
                tokens: vec![],
            });
        }
        let (tokens, spans): (Vec<_>, Vec<_>) =
            lexer_result.into_output().unwrap().into_iter().unzip();

        let func_retriever = Rc::new(|name: String, func_type: FunctionPointerType| {
            if let Some(func_ptr) = module
                .find_function_uuid_by_name(&name, func_type)
                .map(|x| x.uuid())
            {
                Some(func_ptr)
            } else {
                let uuid = Uuid::new_v4();
                match func_type {
                    FunctionPointerType::External => unresolved_external_functions
                        .borrow_mut()
                        .insert(name, uuid),
                    FunctionPointerType::Internal => unresolved_internal_functions
                        .borrow_mut()
                        .insert(name, uuid),
                };
                Some(uuid)
            }
        });

        let uuid_generator = Rc::new(Uuid::new_v4);
        let parser = final_parser();

        let mut state = SimpleState(State::new(registry, func_retriever, uuid_generator));
        let parse_result = parser.parse_with_state(tokens.as_slice(), &mut state);
        if parse_result.has_errors() {
            error!(
                "Parsing errors encountered in file {}:",
                current_path.to_string_lossy()
            );

            let errors = parse_result
                .into_errors()
                .into_iter()
                .map(|e| {
                    let span = e.span();

                    // Convert token span to source span
                    let source_span = SimpleSpan {
                        start: spans[span.start].start,
                        end: spans[span.end - 1].end,
                        context: (),
                    };

                    ParserError {
                        file: Some(current_path.to_string_lossy().to_string()),
                        start: source_span.start,
                        end: source_span.end,
                        message: format!("{}", e.reason()),
                    }
                })
                .collect();
            return Err(Error::ParserErrors {
                errors,
                tokens: tokens.iter().map(|t| format!("{:?}", t)).collect(),
            });
        }

        // Process parsed items
        let items = parse_result.into_output().unwrap();
        for item in items {
            match item {
                Item::Import(path) => {
                    // Push the imported file onto the stack for processing, stop processing current file
                    let import_path = current_path.parent().unwrap().join(&path);
                    debug!("Add file to import list {}", import_path.to_string_lossy());

                    let canonical_import_path = std::fs::canonicalize(&import_path)
                        .map_err(|e| Error::FileNotFound { path, cause: e })
                        .inspect_err(|e| {
                            error!(
                                "An error occurred while canonicalizing the import path: {}",
                                e
                            )
                        })?;
                    stack.push(canonical_import_path);
                }
                Item::Function(mut function) => {
                    debug!("Adding function {:?} to module", function.name);
                    function.normalize_ssa();

                    // Add it to the list functions to be added after verification
                    list_added_internal_functions.push(function);
                }
            }
        }
    }

    // Resolve all function, ensuring that (1) everything is resolved, and
    // (2) unique names are enforced
    let mut resolved_internal_functions: HashMap<Uuid, Uuid> = HashMap::new();
    for (name, uuid) in unresolved_internal_functions.borrow().iter() {
        // Find the function in the list_added_internal_functions
        let matching_functions: Vec<_> = list_added_internal_functions
            .iter()
            .filter(|f| f.name.as_ref() == Some(name))
            .collect();
        if matching_functions.is_empty() {
            error!("Unresolved internal function: {:?}", name);
            return Err(Error::UnresolvedFunction {
                name: name.clone(),
                func_type: FunctionPointerType::Internal,
            });
        } else if matching_functions.len() > 1 {
            error!("Multiple functions found with the same name: {}", name);
            return Err(Error::FunctionAlreadyExists { name: name.clone() });
        }

        let function = matching_functions[0];
        resolved_internal_functions.insert(*uuid, function.uuid);
    }

    // For no external functions cannot be defined by the module as such all unresolved external is treated as an error;
    if !unresolved_external_functions.borrow().is_empty() {
        let names: Vec<String> = unresolved_external_functions
            .borrow()
            .keys()
            .cloned()
            .collect();
        error!("Unresolved external functions: {:?}", names);
        return Err(Error::UnresolvedFunction {
            name: names.join(", "),
            func_type: FunctionPointerType::External,
        });
    }

    // Finally update all the links internally
    for mut func in list_added_internal_functions.into_iter() {
        for (_, block) in func.body.iter_mut() {
            for operands in block
                .terminator
                .operands_mut()
                .chain(block.instructions.iter_mut().flat_map(|x| x.operands_mut()))
            {
                if let Some(func_ptr) = operands
                    .try_as_imm_mut()
                    .and_then(|imm| imm.try_as_func_ptr_mut())
                {
                    match func_ptr {
                        FunctionPointer::Internal(uuid) => {
                            if let Some(new_uuid) = resolved_internal_functions.get(uuid) {
                                *uuid = *new_uuid;
                            }
                        }
                        FunctionPointer::External(_) => {}
                    }
                }
            }
        }

        // Add it to the module
        module.functions.insert(func.uuid, Arc::new(func));
    }

    // Verify module
    if let Err(e) = module.verify() {
        error!("Module verification failed: {}", e);
        return Err(e);
    }

    // Finally, return success
    Ok(())
}

/// Extend a module by parsing a source string.
///
/// Notes: String does not support imports at this time.
///
/// # Arguments
/// - `module`: The module to extend.
/// - `registry`: The type registry to use for type resolution.
/// - `source`: The source string to parse.
///
/// # Returns
/// - `Ok(())` if the module was successfully extended.
///
pub fn extend_module_from_string(
    module: &mut Module,
    registry: &TypeRegistry,
    source: &str,
) -> Result<(), Error> {
    // Lex the source string
    let lexer_result = lexer().parse(source);
    if lexer_result.has_errors() {
        error!("Lexing errors encountered in provided source string:");

        let errors = lexer_result
            .into_errors()
            .into_iter()
            .map(|e| ParserError {
                file: None,
                start: e.span().start,
                end: e.span().end,
                message: e.reason().to_string(),
            })
            .collect();
        return Err(Error::ParserErrors {
            errors,
            tokens: vec![],
        });
    }

    let (tokens, spans): (Vec<_>, Vec<_>) = lexer_result.into_output().unwrap().into_iter().unzip();

    // Final parser, import + function definitions
    let unresolved_internal_functions: RefCell<HashMap<String, Uuid>> = Default::default();
    let unresolved_external_functions: RefCell<HashMap<String, Uuid>> = Default::default();
    let mut list_added_internal_functions = vec![];

    {
        let func_retriever = Rc::new(|name: String, func_type: FunctionPointerType| {
            if let Some(func_ptr) = module
                .find_function_uuid_by_name(&name, func_type)
                .map(|x| x.uuid())
            {
                Some(func_ptr)
            } else {
                let uuid = match func_type {
                    FunctionPointerType::External => *unresolved_external_functions
                        .borrow_mut()
                        .entry(name)
                        .or_insert_with(Uuid::new_v4),
                    FunctionPointerType::Internal => *unresolved_internal_functions
                        .borrow_mut()
                        .entry(name)
                        .or_insert_with(Uuid::new_v4),
                };
                Some(uuid)
            }
        });

        let uuid_generator = Rc::new(Uuid::new_v4);
        let parser = final_parser();

        let mut state = SimpleState(State::new(registry, func_retriever, uuid_generator));
        let parse_result = parser.parse_with_state(tokens.as_slice(), &mut state);
        if parse_result.has_errors() {
            error!("Parsing errors encountered in provided source string:");

            let errors = parse_result
                .into_errors()
                .into_iter()
                .map(|e| {
                    let span = e.span();

                    // Convert token span to source span
                    let source_span = SimpleSpan {
                        start: spans[span.start].start,
                        end: spans[span.end - 1].end,
                        context: (),
                    };

                    ParserError {
                        file: None,
                        start: source_span.start,
                        end: source_span.end,
                        message: format!("{}", e.reason()),
                    }
                })
                .collect();
            return Err(Error::ParserErrors {
                errors,
                tokens: tokens.iter().map(|t| format!("{:?}", t)).collect(),
            });
        }

        // Process parsed items (string source does not support imports)
        let items = parse_result.into_output().unwrap();
        for item in items {
            match item {
                Item::Import(path) => {
                    error!(
                        "Import encountered in string source; imports unsupported in this context: {}",
                        path
                    );

                    let errors = vec![ParserError {
                        file: None,
                        start: 0,
                        end: 0,
                        message: format!(
                            "import statements are not supported when parsing from string: {}",
                            path
                        ),
                    }];
                    return Err(Error::ParserErrors {
                        errors,
                        tokens: tokens.iter().map(|t| format!("{:?}", t)).collect(),
                    });
                }
                Item::Function(mut function) => {
                    debug!("Adding function {:?} to module", function.name);
                    function.normalize_ssa();
                    list_added_internal_functions.push(function);
                }
            }
        }
    } // end of inner scope; drop parser state and func_retriever

    // Resolve all functions: ensure referenced internal functions are defined exactly once
    let mut resolved_internal_functions: HashMap<Uuid, Uuid> = HashMap::new();
    for (name, uuid) in unresolved_internal_functions.borrow().iter() {
        let matching_functions: Vec<_> = list_added_internal_functions
            .iter()
            .filter(|f| f.name.as_ref() == Some(name))
            .collect();
        if matching_functions.is_empty() {
            error!("Unresolved internal function: {:?}", name);
            return Err(Error::UnresolvedFunction {
                name: name.clone(),
                func_type: FunctionPointerType::Internal,
            });
        } else if matching_functions.len() > 1 {
            error!("Multiple functions found with the same name: {}", name);
            return Err(Error::FunctionAlreadyExists { name: name.clone() });
        }

        let function = matching_functions[0];
        resolved_internal_functions.insert(*uuid, function.uuid);
    }

    // External functions cannot be defined in the string source; treat unresolved externals as error
    if !unresolved_external_functions.borrow().is_empty() {
        let names: Vec<String> = unresolved_external_functions
            .borrow()
            .keys()
            .cloned()
            .collect();
        error!("Unresolved external functions: {:?}", names);
        return Err(Error::UnresolvedFunction {
            name: names.join(", "),
            func_type: FunctionPointerType::External,
        });
    }

    // Update all internal function pointer links and insert functions into the module
    // Ensure parser state is dropped to release any immutable borrows on `module`.
    // parser state and func_retriever have been dropped by leaving scope above
    for mut func in list_added_internal_functions.into_iter() {
        for (_, block) in func.body.iter_mut() {
            for operands in block
                .terminator
                .operands_mut()
                .chain(block.instructions.iter_mut().flat_map(|x| x.operands_mut()))
            {
                if let Some(func_ptr) = operands
                    .try_as_imm_mut()
                    .and_then(|imm| imm.try_as_func_ptr_mut())
                {
                    match func_ptr {
                        FunctionPointer::Internal(uuid) => {
                            if let Some(resolved) = resolved_internal_functions.get(uuid) {
                                *uuid = *resolved;
                            }
                        }
                        FunctionPointer::External(_) => {}
                    }
                }
            }
        }

        module.functions.insert(func.uuid, Arc::new(func));
    }

    // Verify module integrity
    if let Err(e) = module.verify() {
        error!("Module verification failed: {}", e);
        return Err(e);
    }

    Ok(())
}

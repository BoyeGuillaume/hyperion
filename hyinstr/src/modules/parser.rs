use std::{
    cell::RefCell,
    collections::{BTreeMap, HashSet},
    hash::Hash,
    path::{Path, PathBuf},
    rc::Rc,
    u16, u64,
};

use bigdecimal::{BigDecimal, Num};
use chumsky::{
    input::ValueInput, label, prelude::*, text::{Char, ascii::ident, digits}
};
use either::Either::{self, Left};
use log::debug;
use num_bigint::{BigInt, Sign};
use smallvec::SmallVec;
use strum::{EnumDiscriminants, EnumIs, EnumTryAs, IntoEnumIterator};
use uuid::Uuid;

use crate::{
    consts::{AnyConst, fp::FConst, int::IConst},
    modules::{
        BasicBlock, CallingConvention, Function, Instruction, Module, Visibility, fp::*, instructions::{HyInstr, HyInstrOp}, int::*, mem::*, meta::*, misc::*, operand::{Label, Name, Operand}, symbol::{FunctionPointer, FunctionPointerType}, terminator::*
    },
    types::{
        AnyType, TypeRegistry, Typeref,
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

    /// UUID parser (prefixed with '@')
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
}

impl Token<'_> {
    pub fn discriminant(&self) -> TokenDiscriminants {
        self.into()
    }
}

fn just_match<'src, I>(
    token: TokenDiscriminants,
) -> impl Parser<'src, I, Token<'src>, extra::Err<Rich<'src, Token<'src>, Span>>> + Clone
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span> + Clone,
{
    any()
        .filter(move |t: &Token| t.discriminant() == token)
        .labelled(format!("token {:?}", token))
}

fn tuple_left<A, B>(t: (A, B)) -> A {
    t.0
}

fn tuple_right<A, B>(t: (A, B)) -> B {
    t.1
}

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

fn bigint_parser<'src>()
-> impl Parser<'src, &'src str, BigInt, extra::Err<Rich<'src, char>>> + Clone {
    let hex_num = just("0x")
        .ignore_then(digits(16).to_slice())
        .map(|x: &str| BigInt::parse_bytes(x.as_bytes(), 16).unwrap());

    let oct_num = just("0o")
        .ignore_then(digits(8).to_slice())
        .map(|x: &str| BigInt::parse_bytes(x.as_bytes(), 8).unwrap());

    let bin_num = just("0b")
        .ignore_then(digits(2).to_slice())
        .map(|x: &str| BigInt::parse_bytes(x.as_bytes(), 2).unwrap());

    let dec_num = digits(10)
        .to_slice()
        .map(|x: &str| BigInt::parse_bytes(x.as_bytes(), 10).unwrap());

    choice((hex_num, oct_num, bin_num, dec_num)).labelled("number")
}

fn u32_parser<'src>() -> impl Parser<'src, &'src str, u32, extra::Err<Rich<'src, char>>> + Clone {
    digits(10)
        .to_slice()
        .validate(|s: &str, extra, emit| match u32::from_str_radix(s, 10) {
            Ok(val) => val,
            Err(e) => {
                emit.emit(Rich::custom(
                    extra.span(),
                    format!("invalid u32 number: {}", e),
                ));
                0u32
            }
        })
        .labelled("u32 number")
}

fn string_parser<'src>() -> impl Parser<'src, &'src str, String, extra::Err<Rich<'src, char>>> + Clone {
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
                .collect::<String>()
        )
        .then_ignore(just('"'))
        .labelled("string literal")
}

fn bigdecimal_parser<'src>()
-> impl Parser<'src, &'src str, BigDecimal, extra::Err<Rich<'src, char>>> + Clone {
    // Simple floating-point parser using BigDecimal
    let sign = any()
        .filter(|&x: &char| x == '+' || x == '-')
        .or_not()
        .map(|opt_sign| opt_sign.unwrap_or('+'))
        .labelled("sign");
    let integer_part = any()
        .filter(|c: &char| c.is_ascii_digit())
        .repeated()
        .labelled("integer part");
    let fractional_part = just('.')
        .ignore_then(any().filter(|c: &char| c.is_ascii_digit()).repeated())
        .labelled("fractional part")
        .or_not();
    let exponent_part = just('e')
        .or(just('E'))
        .ignore_then(
            sign.clone().then(
                any()
                    .filter(|c: &char| c.is_ascii_digit())
                    .repeated()
                    .at_least(1)
                    .labelled("exponent digits"),
            ),
        )
        .labelled("exponent part")
        .or_not();
    sign.then(integer_part)
        .then(fractional_part)
        .then(exponent_part)
        .to_slice()
        .validate(
            |s: &str, extra, emit| match BigDecimal::from_str_radix(s, 10) {
                Ok(val) => val,
                Err(e) => {
                    emit.emit(Rich::custom(
                        extra.span(),
                        format!("invalid floating-point number: {}", e),
                    ));
                    BigDecimal::from(0)
                }
            },
        )
        .labelled("decimal floating-point number")
}

fn identifier_parser<'src>()
-> impl Parser<'src, &'src str, Token<'src>, extra::Err<Rich<'src, char>>> + Clone {
    let base_identifier = chumsky::text::ident()
        .then(
            just(".")
                .ignore_then(chumsky::text::ident().to_slice())
                .repeated()
                .collect::<Vec<_>>(),
        )
        .validate(|(s, other), extra, emit| {
            if s == "void" && other.is_empty() {
                return Token::Void;
            }
            if s == "import" && other.is_empty() {
                return Token::Import;
            }
            if let Some(visibility) = Visibility::from_str(s) {
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
            if let Some(cc) = CallingConvention::from_str(s) {
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
            if let Some(hyinstr_op) = HyInstrOp::from_str(s) {
                return Token::InstrOp(hyinstr_op, other);
            }
            if let Some(terminator_op) = HyTerminatorOp::from_str(s) {
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

            if other.is_empty() {
                let ftype = FType::from_str(s).map(Token::FType);
                let ordering = MemoryOrdering::from_str(s).map(Token::Ordering);

                ftype
                    .or(ordering)
                    .unwrap_or_else(|| Token::Identifier(s, other))
            } else {
                Token::Identifier(s, other)
            }
        })
        .labelled("identifier");

    let itype = just("i")
        .ignore_then(u32_parser())
        .try_map(|width, span| {
            IType::try_new(width).map(Token::IType).ok_or_else(|| {
                Rich::custom(
                    span,
                    format!(
                        "cannot create IType with width {} (must be between {} and {})",
                        width,
                        IType::MIN_BITS,
                        IType::MAX_BITS
                    ),
                )
            })
        })
        .labelled("itype");

    choice((base_identifier, itype))
}

fn register_parser<'src>()
-> impl Parser<'src, &'src str, &'src str, extra::Err<Rich<'src, char>>> + Clone {
    just("%")
        .ignore_then(
            any()
                .filter(|c: &char| c.is_ident_continue())
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
        any().filter(|c: &char| c.is_whitespace()).repeated(),
        comment_parser(),
    ))
    .repeated()
    .ignored()
    .labelled("whitespace or comment")
}

fn lexer<'src>()
-> impl Parser<'src, &'src str, Vec<Spanned<Token<'src>>>, extra::Err<Rich<'src, char>>> {
    choice((
        bigint_parser().map(Token::Number),
        bigdecimal_parser().map(Token::Decimal),
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
        register_parser().map(Token::Register),
        identifier_parser(),
    ))
    .padded_by(ignoring_parser())
    .map_with(|item, extra| (item, extra.span()))
    .repeated()
    .collect::<Vec<_>>()
}

fn primary_basic_type_parser<'src, I>()
-> impl Parser<'src, I, PrimaryBasicType, extra::Err<Rich<'src, Token<'src>, Span>>> + Clone
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span> + Clone,
{
    any()
        .filter(|x: &Token| {
            x.is_i_type()
                || x.is_f_type()
                || x
                    .try_as_identifier_ref()
                    .map(|x| x.1.is_empty() && *x.0 == "ptr")
                    .unwrap_or(false)
        })
        .map(|token| {
            match token {
                Token::IType(itype) => PrimaryBasicType::Int(itype).into(),
                Token::FType(ftype) => PrimaryBasicType::Float(ftype).into(),
                Token::Identifier(s, v) if s == "ptr" && v.is_empty() => {
                    PrimaryBasicType::Ptr(PtrType).into()
                }
                _ => unreachable!(),
            }
        })
}

fn primary_type_parser<'src, I>()
-> impl Parser<'src, I, PrimaryType, extra::Err<Rich<'src, Token<'src>, Span>>> + Clone
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span> + Clone,
{
    let primary_type =
        primary_basic_type_parser().map(|prim_type| prim_type.into());

    // Vector types (e.g., <4 x i32> or <vscale 4 x i32>)
    let vector_type = just(Token::Identifier("vscale", vec![]))
        .or_not()
        .then(
            just_match(TokenDiscriminants::Number)
            .validate(
                |num_span, extra, emit| {
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
                },
            ),
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

fn type_parser<'src, I>(
    registry: &'src TypeRegistry,
) -> impl Parser<'src, I, Typeref, extra::Err<Rich<'src, Token<'src>, Span>>> + Clone
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span> + Clone,
{
    recursive(|tree| {
        // Primary basic types
        let primary_type = primary_type_parser().map(move |prim_type| {
            registry.search_or_insert(prim_type.into())
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
                registry.search_or_insert(array_type.into())
            })
            .labelled("array type");

        // Structure types (e.g., { i32, fp32, [4 x i8] })
        let struct_type = just(Token::Identifier("packed", vec![]))
            .or_not()
            .then(
                tree
                    .clone()
                    .separated_by(just(Token::Comma))
                    .collect::<Vec<_>>()
                    .delimited_by(
                        just(Token::LBrace),
                        just(Token::RBrace),
                    ),
            )
            .map_with(|(packed, element_types), extra| {
                let struct_type = StructType {
                    element_types,
                    packed: packed.is_some(),
                };
                registry.search_or_insert(struct_type.into())
            })
            .labelled("struct type");

        choice((primary_type, array_type, struct_type))
    })
    .labelled("type")
}

fn constant_parser<'src, I>(
    func_retriver: impl Fn(String, FunctionPointerType) -> Option<Uuid> + Clone + 'src,
) -> impl Parser<'src, I, AnyConst, extra::Err<Rich<'src, Token<'src>, Span>>> + Clone
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

            match func_retriver(name.clone(), ftype) {
                Some(uuid) => 
                    match ftype {
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

    choice((itype_const, ftype_const, func_ptr))
}

fn label_parser<'src, I>(
    label_namespace: impl Fn(&str) -> Label + Clone + 'src,
) -> impl Parser<'src, I, Label, extra::Err<Rich<'src, Token<'src>, Span>>> + Clone
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span> + Clone,
{
    just_match(TokenDiscriminants::Identifier)
        .map(move |token| {
            let ident = token.try_as_identifier().unwrap();
            let mut full_name = ident.0.to_string();
            for part in ident.1 {
                full_name.push('.');
                full_name.push_str(part);
            }

            label_namespace(&full_name)
        })
        .labelled("label")
}

fn register_parser_a<'src, I>(
    register_namespace: impl Fn(&str) -> Name + Clone + 'src,
) -> impl Parser<'src, I, Name, extra::Err<Rich<'src, Token<'src>, Span>>> + Clone
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span> + Clone,
{
    just_match(TokenDiscriminants::Register)
        .map(move |token| {
            register_namespace(token.try_as_register().unwrap())
        })
        .labelled("register")
}

fn operand_parser<'src, I>(
    register_namespace: impl Fn(&str) -> Name + Clone + 'src,
    func_retriver: impl Fn(String, FunctionPointerType) -> Option<Uuid> + Clone + 'src,
) -> impl Parser<'src, I, Operand, extra::Err<Rich<'src, Token<'src>, Span>>> + Clone
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span> + Clone,
{
    let reg_parser = register_parser_a(register_namespace).map(Operand::Reg);
    let const_parser = constant_parser(func_retriver)
        .map(Operand::Imm)
        .labelled("immediate operand");

    choice((reg_parser, const_parser))
}

fn parse_instruction<'src, I>(
    register_namespace: impl Fn(&str) -> Name + Clone + 'src,
    label_namespace: impl Fn(&str) -> Label + Clone + 'src,
    func_retriver: impl Fn(String, FunctionPointerType) -> Option<Uuid> + Clone + 'src,
    type_registry: &'src TypeRegistry,
) -> impl Parser<'src, I, HyInstr, extra::Err<Rich<'src, Token<'src>, Span>>> + Clone
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span> + Clone,
{
    let operand_parser = choice((
        /* Use by most instructions */
        operand_parser(register_namespace.clone(), func_retriver.clone())
            .separated_by(just(Token::Comma))
            .collect::<Vec<_>>()
            .map(Either::Left),

        /* Use by phi instructions */
        operand_parser(register_namespace.clone(), func_retriver.clone())
            .then(
                label_parser(move |s| label_namespace(s))
            )
            .separated_by(just(Token::Comma))
            .collect::<Vec<_>>()
            .delimited_by(just(Token::LBracket), just(Token::RBracket))
            .map(Either::Right),
    ));

    just_match(TokenDiscriminants::Register)
        .map(|x| x.try_as_register().unwrap())
        .then_ignore(just(Token::Colon))
        .then(type_parser(type_registry))
        .then_ignore(just(Token::Equals))
        .or_not()
        .then(
            just_match(TokenDiscriminants::InstrOp)
                .map(|x| x.try_as_instr_op().unwrap()),
        )
        .then(operand_parser)
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

                align.to_u32_digits().1.into_iter().next().unwrap()
            })
            .or_not(),
        )
        .validate(move |(elem, align), extra, emit| {
            let ((destination, op), operand) = elem;
            let (op, variant) = op;
            let dest_and_ty = if let Some((dest, ty)) = destination {
                Some((register_namespace(dest), ty))
            } else {
                None
            };

            if op != HyInstrOp::Phi && matches!(operand, Either::Right(_)) {
                emit.emit(Rich::custom(
                    extra.span(),
                    format!(
                        "syntax error for {} instruction: only 'phi' instructions can use the [operand, label] syntax, use operands separated by commas instead",
                        op.opname()
                    )
                ));

                return 
                    HyInstr::MetaAssert(MetaAssert { condition: Operand::Imm(IConst::from(1u64).into()) })
                ;
            }
            else if op == HyInstrOp::Phi && matches!(operand, Either::Left(_)) {
                emit.emit(Rich::custom(
                    extra.span(),
                    format!(
                        "syntax error for phi instruction: expected [operand, label] pairs, got operands separated by commas instead",
                    )
                ));

                return 
                    HyInstr::MetaAssert(MetaAssert { condition: Operand::Imm(IConst::from(1u64).into()) })
                ;
            }

            if op.has_variant() != variant.is_empty() {
                emit.emit(Rich::custom(
                    extra.span(),
                    format!(
                        "syntax error for {} instruction: expected {} variant operands, got {}",
                        op.opname(),
                        if op.has_variant() { "variant" } else { "no variant" },
                        if variant.is_empty() { "no variant" } else { "variant" }
                    ),
                ));

                return 
                    HyInstr::MetaAssert(MetaAssert { condition: Operand::Imm(IConst::from(1u64).into()) })
                ;
            }

            if op.has_variant() {
                let num_variant_operands = match op {
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

                    return 
                        HyInstr::MetaAssert(MetaAssert { condition: Operand::Imm(IConst::from(1u64).into()) })
                    ;
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

                    return 
                        HyInstr::MetaAssert(MetaAssert { condition: Operand::Imm(IConst::from(1u64).into()) });
                    
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

            match op {
                HyInstrOp::IAdd | HyInstrOp::ISub | HyInstrOp::IMul => {
                    let [lhs, rhs] = operand.unwrap_left().try_into().unwrap();
                    let (dest, ty) = dest_and_ty.unwrap();
                    let variant = match OverflowSignednessPolicy::from_str(&variant[0]) {
                        Some(variant) => variant,
                        None => {
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
                    let signedness = match IntegerSignedness::from_str(&variant[0]) {
                        Some(variant) => variant,
                        None => {
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
                    let variant = match IShiftVariant::from_str(&variant[0]) {
                        Some(variant) => variant,
                        None => {
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
                HyInstrOp::IAnd |
                HyInstrOp::IOr |
                HyInstrOp::IXor |
                HyInstrOp::INot |
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
                    let variant = match FCmpVariant::from_str(&variant[0]) {
                        Some(variant) => variant,
                        None => {
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

                    let variant = match ICmpVariant::from_str(&variant[0]) {
                        Some(variant) => variant,
                        None => {
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
                HyInstrOp::MLoad => todo!(),
                HyInstrOp::MStore => todo!(),
                HyInstrOp::MAlloca => {
                    let [count] = operand.unwrap_left().try_into().unwrap();
                    let (dest, ty) = dest_and_ty.unwrap();
                    
                    MAlloca { dest, ty, count, alignment: align }.into()
                }
                HyInstrOp::MGetElementPtr => {
                    let mut indices = operand.unwrap_left();
                    let (dest, ty) = dest_and_ty.unwrap();
                    
                    if indices.is_empty() {
                        emit.emit(Rich::custom(
                            extra.span(),
                            format!(
                                "arity mismatch for {} instruction: expected at least 1 operand for indices, got 0",
                                op.opname(),
                            ),
                        ));

                        return 
                            HyInstr::MetaAssert(MetaAssert { condition: Operand::Imm(IConst::from(1u64).into()) })
                       ;
                    }

                    let base = indices.remove(0);

                    MGetElementPtr { dest, ty, base, indices }.into()
                }
                HyInstrOp::Invoke => todo!(),
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
                    let variant = match CastVariant::from_str(&variant[0]) {
                        Some(op) => op,
                        None => {
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
                HyInstrOp::MetaAssert => {
                    let [condition] = operand.unwrap_left().try_into().unwrap();

                    MetaAssert { condition }.into()
                }
                HyInstrOp::MetaAssume => {
                    let [condition] = operand.unwrap_left().try_into().unwrap();

                    MetaAssume { condition }.into()
                }
                HyInstrOp::MetaProb => {
                    let (dest, ty) = dest_and_ty.unwrap();
                    let variant = match MetaProbVariant::from_str(&variant[0]) {
                        Some(op) => op,
                        None => {
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

                        return 
                            HyInstr::MetaAssert(MetaAssert { condition: Operand::Imm(IConst::from(1u64).into()) })
                        ;
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
                        MetaProbVariant::ProbabilityReachability => MetaProbOperand::ProbabilityReachability,
                        MetaProbVariant::ExpectedIterations => MetaProbOperand::ExpectedIterations,
                    };

                    MetaProb { dest, ty, operand }.into()
                }
            }
        })
}

fn parse_terminator<'src, I>(
    register_namespace: impl Fn(&str) -> Name + Clone + 'src,
    label_namespace: impl Fn(&str) -> Label + Clone + 'src,
    func_retriver: impl Fn(String, FunctionPointerType) -> Option<Uuid> + Clone + 'src,
) -> impl Parser<'src, I, HyTerminator, extra::Err<Rich<'src, Token<'src>, Span>>> + Clone
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span> + Clone,
{
    let branch = just(Token::TerminatorOp(HyTerminatorOp::Branch))
        .then(operand_parser(register_namespace.clone(), func_retriver.clone())
    )
    .then_ignore(just(Token::Comma))
    .then(
        label_parser(label_namespace.clone())
    )
    .then(
        label_parser(label_namespace.clone())
    ).map(|(((op, cond), target_true), target_false)| {
        Branch {
            cond: cond,
            target_true: target_true,
            target_false: target_false,
        }.into()
    });

    let trap = just(Token::TerminatorOp(HyTerminatorOp::Trap))
        .to(Trap.into());

    let jump = just(Token::TerminatorOp(HyTerminatorOp::Jump))
        .ignore_then(
            label_parser(label_namespace.clone())
        )
        .map(|target| {
            Jump {
                target: target,
            }.into()
        });
    
    let ret = just(Token::TerminatorOp(HyTerminatorOp::Ret))
        .ignore_then(
            operand_parser(register_namespace.clone(), func_retriver.clone()).map(Either::Left)
            .or(just(Token::Void).map(Either::Right))
        )
        .map(|operand| {
            Ret {
                value: operand.left(),
            }.into()
        });

    choice((
        branch,
        trap,
        jump,
        ret,
    )).boxed()
}


fn parse_function<'src, I>(
    register_namespace: impl Fn(&str) -> Name + Clone + 'src,
    label_namespace: impl Fn(&str) -> Label + Clone + 'src,
    func_retriver: impl Fn(String, FunctionPointerType) -> Option<Uuid> + Clone + 'src,
    type_registry: &'src TypeRegistry,
    uuid_generator: impl Fn(&str) -> Uuid + Clone + 'src,
) -> impl Parser<'src, I, Function, extra::Err<Rich<'src, Token<'src>, Span>>> + Clone
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span> + Clone,
{
    let block_label = label_parser(label_namespace.clone())
        .then_ignore(just(Token::Colon));

    let block = block_label
        .then(
            parse_instruction(
                register_namespace.clone(),
                label_namespace.clone(),
                func_retriver.clone(),
                type_registry,
            )
            .repeated()
            .collect::<Vec<_>>(),
        )
        .then(
            parse_terminator(
                register_namespace.clone(),
                label_namespace.clone(),
                func_retriver.clone(),
            ),
        )
        .map(|((label, instructions), terminator)| {
            BasicBlock {
                label: label,
                instructions,
                terminator: terminator,
            }
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
                        emit.emit(Rich::custom(
                            extra.span(),
                            "duplicate visibility metadata",
                        ));
                    }
                    seen_visibility = true;
                }
            }

            meta_args
        });

    let arglist = 
        register_parser_a(register_namespace.clone())
        .then_ignore(just(Token::Colon))
        .then(type_parser(type_registry))
        .separated_by(just(Token::Comma))
        .collect::<Vec<_>>()
        .delimited_by(
            just(Token::LParen),
            just(Token::RParen),
        );

    just(Token::Identifier("define", vec![]))
    .ignore_then(type_parser(type_registry).map(Either::Left).or(just(Token::Void).map(Either::Right)))
    .then(meta_arguments)
    .then(
        just_match(TokenDiscriminants::Register)
            .map(|x| x.try_as_register().unwrap()),
    )
    .then(arglist)
    .then(
        block.repeated()
        .collect::<Vec<_>>()
        .delimited_by(
            just(Token::LBrace),
            just(Token::RBrace),
        )
    )
    .map(move |((((ty, meta), func_name), params), blocks)| {
        let uuid = uuid_generator(func_name);
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
            visibility: visibility,
            cconv: cconv,
            wildcard_types: Default::default(),
            meta_function: false,
        };

        func
    })
    .labelled("function definition")
}

fn import_parser<'src, I>() -> impl Parser<'src, I, String, extra::Err<Rich<'src, Token<'src>, Span>>> + Clone
where
    I: ValueInput<'src, Token = Token<'src>, Span = Span> + Clone,
{
    // spanned_match_identifier("import")
    just_match(TokenDiscriminants::Import)
        .ignore_then(
            just_match(TokenDiscriminants::StringLiteral)
                .map(|token| token.try_as_string_literal().unwrap()),
        )
        .labelled("import statement")
}

/// ===================================================================
/// =================== OLD PARSERS BELOW THIS LINE ===================
/// ===================================================================
pub fn function_parser<'src>(
    func_retriver: impl Fn(String, FunctionPointerType) -> Option<Uuid> + Clone + 'src,
    registry: &'src TypeRegistry,
    uuid: Uuid,
) -> impl Parser<'src, &'src str, crate::modules::Function, extra::Err<Rich<'src, char>>> {
    todo()
}

enum ModuleItem {
    Import(String),
    Function(Function),
}

fn extend_module<A: Clone + Eq + Hash>(
    module: &mut Module,
    registry: &TypeRegistry,
    from_string: impl Fn(&str) -> A,
    to_string: impl Fn(A) -> String,
    relative_to: impl Fn(A, A) -> A,
    include: impl Fn(A) -> Result<String, Error>,
    initial: A,
) -> Result<(), Error> {
    todo!()
}

pub fn extend_module_from_string(
    module: &mut Module,
    registry: &TypeRegistry,
    source: &str,
) -> Result<(), Error> {
    extend_module(
        module,
        registry,
        |_| panic!("Cannot resolve relative paths from string source"),
        |a| a,
        |_, _| panic!("Cannot resolve relative paths from string source"),
        |a| {
            debug!("Reading source from string source");
            Ok(a)
        },
        source.to_string(),
    )
}

pub fn extend_module_from_path(
    module: &mut Module,
    registry: &TypeRegistry,
    path: impl AsRef<Path>,
) -> Result<(), Error> {
    // Canonicalize the path
    let canonical_path = std::fs::canonicalize(&path).map_err(|e| Error::FileNotFound {
        path: path.as_ref().to_string_lossy().to_string(),
        cause: e,
    })?;

    extend_module(
        module,
        registry,
        |s: &str| PathBuf::from(s),
        |a: PathBuf| a.to_string_lossy().to_string(),
        |base: PathBuf, relative: PathBuf| base.parent().unwrap().join(relative),
        |a: PathBuf| {
            debug!("Reading source file at path: {}", a.to_string_lossy());
            std::fs::read_to_string(&a)
                .map_err(|e| Error::FileNotFound {
                    path: a.to_string_lossy().to_string(),
                    cause: e,
                })
                .inspect_err(|e| {
                    log::error!("An error occurred while reading the source file: {}", e);
                })
        },
        canonical_path,
    )
}

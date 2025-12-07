use std::{
    cell::RefCell,
    collections::{BTreeMap, HashSet},
    path::{Path, PathBuf},
    rc::Rc,
    u16, u64,
};

use bigdecimal::{BigDecimal, Num};
use chumsky::{
    prelude::*,
    text::{ascii::ident, digits},
};
use either::Either;
use log::debug;
use num_bigint::BigInt;
use smallvec::SmallVec;
use strum::IntoEnumIterator;
use uuid::Uuid;

use crate::{
    consts::{AnyConst, fp::FConst, int::IConst},
    modules::{
        BasicBlock, CallingConvention, Function, Instruction, Module,
        fp::*,
        instructions::HyInstr,
        int::*,
        mem::*,
        misc::*,
        operand::{Label, Name, Operand},
        symbol::{FunctionPointer, FunctionPointerType},
        terminator::{CBranch, Jump, Ret, Terminator},
    },
    types::{
        AnyType, TypeRegistry, Typeref,
        aggregate::{ArrayType, StructType},
        primary::{FType, IType, PrimaryBasicType, PtrType, VcSize, VcType},
    },
    utils::{Error, ParserError},
};

fn whitespace<'src>() -> impl Parser<'src, &'src str, (), extra::Err<Rich<'src, char>>> + Clone {
    any()
        .filter(|c: &char| c.is_whitespace())
        .repeated()
        .at_least(1)
        .ignored()
        .labelled("whitespace")
}

fn itype_parser<'src>() -> impl Parser<'src, &'src str, IType, extra::Err<Rich<'src, char>>> + Clone
{
    just("i")
        .ignore_then(digits(10).to_slice().try_map(|digits, span| {
            // 1. Attempt to parse digits into a usize
            let width: u32 = match u32::from_str_radix(digits, 10) {
                Ok(w) => w,
                Err(_) => {
                    return Err(Rich::custom(span, {
                        format!("invalid integer type width: {}", digits)
                    }));
                }
            };

            // 2. Validate that the width is a positive non-zero value
            if width < IType::MIN_BITS {
                return Err(Rich::custom(span, {
                    format!(
                        "minimum integer type width is {}, got {}",
                        IType::MIN_BITS,
                        width
                    )
                }));
            }

            // 3. Check validity according to your type system rules
            if width > IType::MAX_BITS {
                return Err(Rich::custom(span, {
                    format!(
                        "maximum integer type width is {}, got {}",
                        IType::MAX_BITS,
                        width
                    )
                }));
            }

            Ok(IType::new(width))
        }))
        .labelled("integer type")
}

fn ftype_parser<'src>() -> impl Parser<'src, &'src str, FType, extra::Err<Rich<'src, char>>> + Clone
{
    choice((
        just("fp16").to(FType::Fp16),
        just("half").to(FType::Fp16),
        just("bf16").to(FType::Bf16),
        just("bfloat").to(FType::Bf16),
        just("fp32").to(FType::Fp32),
        just("float").to(FType::Fp32),
        just("fp64").to(FType::Fp64),
        just("double").to(FType::Fp64),
        just("fp128").to(FType::Fp128),
        just("x86_fp80").to(FType::X86Fp80),
        just("ppc_fp128").to(FType::PPCFp128),
    ))
    .labelled("floating-point type")
}

fn icmp_op_parser<'src>()
-> impl Parser<'src, &'src str, ICmpOp, extra::Err<Rich<'src, char>>> + Clone {
    ident()
        .validate(|s: &str, extra, emit| match ICmpOp::from_str(s) {
            Some(op) => op,
            None => {
                emit.emit(Rich::custom(
                    extra.span(),
                    format!(
                        "unknown integer comparison operator: {} (expected one of: {})",
                        s,
                        ICmpOp::iter()
                            .map(|x| x.to_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                ));
                ICmpOp::Eq
            }
        })
        .labelled("integer comparison operator")
}

fn fcmp_op_parser<'src>()
-> impl Parser<'src, &'src str, FCmpOp, extra::Err<Rich<'src, char>>> + Clone {
    ident()
        .validate(|s: &str, extra, emit| match FCmpOp::from_str(s) {
            Some(op) => op,
            None => {
                emit.emit(Rich::custom(
                    extra.span(),
                    format!(
                        "unknown floating-point comparison operator: {} (expected one of: {})",
                        s,
                        FCmpOp::iter()
                            .map(|x| x.to_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                ));
                FCmpOp::One
            }
        })
        .labelled("floating-point comparison operator")
}

fn ordering_parser<'src>()
-> impl Parser<'src, &'src str, MemoryOrdering, extra::Err<Rich<'src, char>>> + Clone {
    ident()
        .validate(|s: &str, extra, emit| match MemoryOrdering::from_str(s) {
            Some(ordering) => ordering,
            None => {
                emit.emit(Rich::custom(
                    extra.span(),
                    format!(
                        "unknown memory ordering: {} (expected one of: {})",
                        s,
                        MemoryOrdering::iter()
                            .map(|x| x.to_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                ));
                MemoryOrdering::Acq
            }
        })
        .labelled("memory ordering")
}

fn cast_op_parser<'src>()
-> impl Parser<'src, &'src str, CastOp, extra::Err<Rich<'src, char>>> + Clone {
    ident()
        .validate(|s: &str, extra, emit| match CastOp::from_str(s) {
            Some(op) => op,
            None => {
                emit.emit(Rich::custom(
                    extra.span(),
                    format!(
                        "unknown cast operator: {} (expected one of: {})",
                        s,
                        CastOp::iter()
                            .map(|x| x.to_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                ));
                CastOp::Trunc
            }
        })
        .labelled("cast operator")
}

fn ishift_op_parser<'src>()
-> impl Parser<'src, &'src str, IShiftOp, extra::Err<Rich<'src, char>>> + Clone {
    ident()
        .validate(|s: &str, extra, emit| match IShiftOp::from_str(s) {
            Some(op) => op,
            None => {
                emit.emit(Rich::custom(
                    extra.span(),
                    format!(
                        "unknown integer shift operator: {} (expected one of: {})",
                        s,
                        IShiftOp::iter()
                            .map(|x| x.to_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                ));
                IShiftOp::Lsl
            }
        })
        .labelled("integer shift operator")
}

fn tptr_parser<'src>() -> impl Parser<'src, &'src str, PtrType, extra::Err<Rich<'src, char>>> + Clone
{
    just("ptr").to(PtrType).labelled("function pointer type")
}

fn bigint_parser<'src>()
-> impl Parser<'src, &'src str, BigInt, extra::Err<Rich<'src, char>>> + Clone {
    choice((
        // Hexadecimal
        just("0x")
            .ignore_then(
                text::digits(16)
                    .at_least(1)
                    .collect::<String>()
                    .try_map(|s, span| {
                        BigInt::parse_bytes(s.as_bytes(), 16).ok_or_else(|| {
                            Rich::custom(span, format!("invalid hexadecimal number: {}", s))
                        })
                    }),
            )
            .labelled("hexadecimal number"),
        // Decimal
        digits(10)
            .at_least(1)
            .collect::<String>()
            .try_map(|s, span| {
                BigInt::parse_bytes(s.as_bytes(), 10)
                    .ok_or_else(|| Rich::custom(span, format!("invalid decimal number: {}", s)))
            })
            .labelled("decimal number"),
    ))
}

fn primary_type_parser<'src>()
-> impl Parser<'src, &'src str, PrimaryBasicType, extra::Err<Rich<'src, char>>> + Clone {
    choice((
        itype_parser().map(PrimaryBasicType::Int),
        ftype_parser().map(PrimaryBasicType::Float),
        tptr_parser().map(PrimaryBasicType::Ptr),
    ))
    .labelled("primitive type")
}

fn type_parser<'src>(
    registry: &'src TypeRegistry,
) -> impl Parser<'src, &'src str, Typeref, extra::Err<Rich<'src, char>>> {
    recursive(|tree| {
        // Primitive type (itype, ftype)
        let primary_type = primary_type_parser().map(|ty| registry.search_or_insert(ty.into()));

        // Array type (e.g., [N x T])
        let vector_array_base = bigint_parser()
            .labelled("number")
            .validate(|elem, extra, emit| {
                if elem <= BigInt::ZERO {
                    emit.emit(Rich::custom(
                        extra.span(),
                        format!("array/vector size must be positive, got {}", elem),
                    ));
                    1u16
                } else if elem > BigInt::from(u16::MAX) {
                    emit.emit(Rich::custom(
                        extra.span(),
                        format!(
                            "array/vector size {} exceeds maximum supported size {}",
                            elem,
                            u16::MAX
                        ),
                    ));
                    u16::MAX
                } else {
                    elem.iter_u64_digits().next().unwrap() as u16
                }
            })
            .padded()
            .then_ignore(just("x"));

        let array_type = just("[")
            .ignore_then(
                vector_array_base
                    .clone()
                    .then(tree.clone().padded())
                    .then_ignore(just("]")),
            )
            .map(|(size, elem_type)| {
                registry.search_or_insert(AnyType::Array(ArrayType {
                    ty: elem_type,
                    num_elements: size,
                }))
            })
            .labelled("array type");

        let vc_type = just("<")
            .ignore_then(just("vscale").padded().or_not())
            .then(
                vector_array_base
                    .then(primary_type_parser().padded())
                    .then_ignore(just(">")),
            )
            .map(|(is_scalable, (size, ty))| {
                registry.search_or_insert(AnyType::Primary(
                    VcType {
                        ty,
                        size: if is_scalable.is_some() {
                            VcSize::Scalable(size)
                        } else {
                            VcSize::Fixed(size)
                        },
                    }
                    .into(),
                ))
            });

        // Struct type (e.g., { T1, T2, T3 })
        let core_struct_type = tree
            .padded()
            .separated_by(just(",").padded())
            .collect::<Vec<_>>()
            .delimited_by(just("{"), just("}"));

        let struct_type = core_struct_type
            .clone()
            .map(|elements| {
                registry.search_or_insert(AnyType::Struct(StructType {
                    element_types: elements,
                    packed: false,
                }))
            })
            .labelled("structure type");

        let packed_struct_type = core_struct_type
            .delimited_by(just("<"), just(">"))
            .map(|elements| {
                registry.search_or_insert(AnyType::Struct(StructType {
                    element_types: elements,
                    packed: true,
                }))
            })
            .labelled("packed structure type");

        // vector_type
        choice((
            primary_type,
            struct_type,
            packed_struct_type,
            array_type,
            vc_type,
        ))
        .labelled("type")
    })
}

fn maybe_type_parser<'src>(
    registry: &'src TypeRegistry,
) -> impl Parser<'src, &'src str, Option<Typeref>, extra::Err<Rich<'src, char>>> {
    choice((
        type_parser(registry).map(Some),
        just("void").to(None).labelled("void type"),
    ))
}

fn uuid_parser<'src>() -> impl Parser<'src, &'src str, uuid::Uuid, extra::Err<Rich<'src, char>>> {
    // UUID parser in standard 8-4-4-4-12 format
    let hex_digit = any()
        .filter(|c: &char| c.is_ascii_hexdigit())
        .labelled("hexadecimal digit");
    hex_digit
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

fn iconst_parser<'src>() -> impl Parser<'src, &'src str, IConst, extra::Err<Rich<'src, char>>> {
    itype_parser()
        .then_ignore(whitespace())
        .then(bigint_parser())
        .validate(|(itype, value), extra, emit| {
            if itype.fits_value(&value) {
                IConst { ty: itype, value }
            } else {
                emit.emit(Rich::custom(
                    extra.span(),
                    format!(
                        "integer constant value {} does not fit in type {} (max {})",
                        value,
                        itype,
                        itype.max_value().unwrap_or(u64::MAX)
                    ),
                ));
                IConst { ty: itype, value }
            }
        })
        .labelled("integer constant")
}

fn decimal_query<'src>() -> impl Parser<'src, &'src str, BigDecimal, extra::Err<Rich<'src, char>>> {
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

fn fp_parser<'src>() -> impl Parser<'src, &'src str, FConst, extra::Err<Rich<'src, char>>> {
    ftype_parser()
        .then_ignore(whitespace())
        .then(decimal_query())
        .map(|(ty, value)| FConst { ty, value })
        .labelled("floating-point constant")
}

fn func_ptr_parser<'src>(
    func_retriver: impl Fn(String, FunctionPointerType) -> Option<Uuid> + 'src,
) -> impl Parser<'src, &'src str, FunctionPointer, extra::Err<Rich<'src, char>>> {
    let named_func = just("%")
        .ignore_then(
            any()
                .filter(|c: &char| c.is_ascii_alphanumeric() || *c == '_')
                .repeated()
                .collect::<String>()
                .labelled("function identifier"),
        )
        .labelled("function pointer")
        .map(Either::Left);

    let uuid_func = just("@")
        .ignore_then(uuid_parser())
        .labelled("function UUID")
        .map(Either::Right);

    tptr_parser()
        .then_ignore(whitespace())
        .ignore_then(just("external").then_ignore(whitespace()).or_not())
        .then(choice((named_func, uuid_func)))
        .validate(move |(is_external, name), extra, emit| {
            let kind = if is_external.is_some() {
                FunctionPointerType::External
            } else {
                FunctionPointerType::Internal
            };

            let uuid = match name {
                Either::Left(func_name) => match func_retriver(func_name.clone(), kind) {
                    Some(uuid) => uuid,
                    None => {
                        emit.emit(Rich::custom(
                            extra.span(),
                            format!("undefined function name: {}", func_name),
                        ));
                        Uuid::nil()
                    }
                },
                Either::Right(uuid) => uuid,
            };

            match kind {
                FunctionPointerType::Internal => FunctionPointer::Internal(uuid),
                FunctionPointerType::External => FunctionPointer::External(uuid),
            }
        })
}

fn label_parser<'src>(
    named_label: impl Fn(String) -> Label + Clone + 'src,
) -> impl Parser<'src, &'src str, Label, extra::Err<Rich<'src, char>>> + Clone {
    chumsky::text::ascii::ident()
        .map(move |s: &str| named_label(s.to_string()))
        .labelled("label")
}

fn percent_name_parser<'src>()
-> impl Parser<'src, &'src str, String, extra::Err<Rich<'src, char>>> + Clone {
    just("%")
        .ignore_then(
            any()
                .filter(|c: &char| c.is_ascii_alphanumeric() || *c == '_')
                .repeated()
                .collect::<String>()
                .labelled("identifier"),
        )
        .labelled("name")
}

fn register_parser<'src>(
    named_name: impl Fn(String) -> Name + 'src,
) -> impl Parser<'src, &'src str, Name, extra::Err<Rich<'src, char>>> {
    percent_name_parser()
        .labelled("register")
        .map(move |x| named_name(x))
}

fn operand_parser<'src>(
    func_retriver: impl Fn(String, FunctionPointerType) -> Option<Uuid> + 'src,
    named_name: impl Fn(String) -> Name + 'src,
    label_namer: impl Fn(String) -> Label + Clone + 'src,
) -> impl Parser<'src, &'src str, Operand, extra::Err<Rich<'src, char>>> {
    let reg_parser = register_parser(named_name).map(Operand::Reg);

    let imm_parser = choice((
        iconst_parser().map(|x| Operand::Imm(x.into())),
        fp_parser().map(|x| Operand::Imm(x.into())),
        func_ptr_parser(func_retriver).map(|x| Operand::Imm(x.into())),
    ))
    .labelled("immediate");

    let lbl_parser = label_parser(label_namer.clone()).map(Operand::Lbl);

    choice((reg_parser, imm_parser, lbl_parser))
}

fn instruction_dest_parser<'src>(
    named_name: impl Fn(String) -> Name + Clone + 'src,
) -> impl Parser<'src, &'src str, Name, extra::Err<Rich<'src, char>>> + Clone {
    percent_name_parser()
        .padded()
        .then_ignore(just('='))
        .padded()
        .map(move |s: String| named_name(s))
        .labelled("instruction destination")
}

struct TplLabelOperand {
    label: Label,
    operand: Operand,
}

impl<const N: usize> chumsky::container::Container<Operand> for SmallVec<Operand, N> {
    fn with_capacity(n: usize) -> Self {
        SmallVec::with_capacity(n)
    }

    fn push(&mut self, item: Operand) {
        SmallVec::push(self, item)
    }
}

impl<const N: usize> chumsky::container::Container<TplLabelOperand>
    for SmallVec<TplLabelOperand, N>
{
    fn with_capacity(n: usize) -> Self {
        SmallVec::with_capacity(n)
    }

    fn push(&mut self, item: TplLabelOperand) {
        SmallVec::push(self, item)
    }
}

#[derive(Clone)]
struct CtxA<'src, F1, F2, F3>
where
    F1: Fn(String, FunctionPointerType) -> Option<Uuid> + Clone,
    F2: Fn(String) -> Name + Clone,
    F3: Fn(String) -> Label + Clone,
{
    func_retriver: F1,
    named_name: F2,
    label_namer: F3,
    registry: &'src TypeRegistry,
}

fn parse_simple_instruction<'src, U, F1, F2, F3>(
    ctx: CtxA<'src, F1, F2, F3>,
    opname: &'static str,
    num_operand: usize,
    parser: impl Parser<'src, &'src str, U, extra::Err<Rich<'src, char>>>,
) -> impl Parser<'src, &'src str, (Name, Typeref, U, SmallVec<Operand, 2>), extra::Err<Rich<'src, char>>>
where
    F1: Fn(String, FunctionPointerType) -> Option<Uuid> + Clone + 'src,
    F2: Fn(String) -> Name + Clone + 'src,
    F3: Fn(String) -> Label + Clone + 'src,
{
    instruction_dest_parser(ctx.named_name.clone())
        .padded()
        .then_ignore(just(opname))
        .then_ignore(whitespace())
        .then(parser.padded())
        .then(type_parser(ctx.registry).padded())
        .then(
            operand_parser(
                ctx.func_retriver.clone(),
                ctx.named_name.clone(),
                ctx.label_namer.clone(),
            )
            .padded()
            .separated_by(just(","))
            .exactly(num_operand)
            .collect::<SmallVec<Operand, 2>>(),
        )
        .map(|(((dest, custom), ty), operands)| (dest, ty, custom, operands))
}

fn parse_instruction<'src, F1, F2, F3>(
    ctx: CtxA<'src, F1, F2, F3>,
) -> impl Parser<'src, &'src str, HyInstr, extra::Err<Rich<'src, char>>>
where
    F1: Fn(String, FunctionPointerType) -> Option<Uuid> + Clone + 'src,
    F2: Fn(String) -> Name + Clone + 'src,
    F3: Fn(String) -> Label + Clone + 'src,
{
    let overflow_policy_parser = choice((
        just("panic").to(OverflowPolicy::Panic),
        just("wrap").to(OverflowPolicy::Wrap),
        just("saturate").to(OverflowPolicy::Saturate),
    ))
    .labelled("overflow policy");

    let integer_signedness_parser = choice((
        just("signed").to(IntegerSignedness::Signed),
        just("unsigned").to(IntegerSignedness::Unsigned),
    ))
    .labelled("integer signedness");

    macro_rules! define_op {
        (
            binary policy signedness,
            $opname:ident,
            $actual:ident
        ) => {
            let $opname = parse_simple_instruction(
                ctx.clone(),
                stringify!($opname),
                2,
                overflow_policy_parser
                    .padded()
                    .then(integer_signedness_parser),
            )
            .map(|(dest, ty, (overflow_policy, signess), mut operands)| {
                HyInstr::$actual($actual {
                    dest,
                    ty,
                    lhs: operands.remove(0),
                    rhs: operands.remove(0),
                    overflow: overflow_policy,
                    signedness: signess,
                })
            });
        };
        (
            binary signedness,
            $opname:ident,
            $actual:ident
        ) => {
            let $opname = parse_simple_instruction(
                ctx.clone(),
                stringify!($opname),
                2,
                integer_signedness_parser,
            )
            .map(|(dest, ty, signedness, mut operands)| {
                HyInstr::$actual($actual {
                    dest,
                    ty,
                    lhs: operands.remove(0),
                    rhs: operands.remove(0),
                    signedness,
                })
            });
        };
        (
            binary,
            $opname:ident,
            $actual:ident
        ) => {
            let $opname = parse_simple_instruction(ctx.clone(), stringify!($opname), 2, empty())
                .map(|(dest, ty, _, mut operands)| {
                    HyInstr::$actual($actual {
                        dest,
                        ty,
                        lhs: operands.remove(0),
                        rhs: operands.remove(0),
                    })
                });
        };
        (
            unary,
            $opname:ident,
            $actual:ident
        ) => {
            let $opname = parse_simple_instruction(ctx.clone(), stringify!($opname), 1, empty())
                .map(|(dest, ty, _, mut operands)| {
                    HyInstr::$actual($actual {
                        dest,
                        ty,
                        value: operands.remove(0),
                    })
                });
        };
    }

    // == Integer operations ==
    define_op!(binary policy signedness, iadd, IAdd);
    define_op!(binary policy signedness, isub, ISub);
    define_op!(binary policy signedness, imul, IMul);
    define_op!(binary signedness, idiv, IDiv);
    define_op!(binary signedness, irem, IRem);
    define_op!(binary, iand, IAnd);
    define_op!(binary, ior, IOr);
    define_op!(binary, ixor, IXor);
    define_op!(binary, iimplies, IImplies);
    define_op!(binary, iequiv, IEquiv);

    define_op!(unary, ineg, INeg);
    define_op!(unary, inot, INot);

    let icmp = parse_simple_instruction(ctx.clone(), "icmp", 2, icmp_op_parser()).map(
        |(dest, ty, op, mut operands)| {
            HyInstr::ICmp(ICmp {
                dest,
                ty,
                lhs: operands.remove(0),
                rhs: operands.remove(0),
                op,
            })
        },
    );

    let isht = parse_simple_instruction(ctx.clone(), "ishift", 2, ishift_op_parser()).map(
        |(dest, ty, op, mut operands)| {
            HyInstr::ISht(ISht {
                dest,
                ty,
                lhs: operands.remove(0),
                rhs: operands.remove(0),
                op,
            })
        },
    );

    // == Floating-point operations ==
    let fcmp = parse_simple_instruction(ctx.clone(), "fcmp", 2, fcmp_op_parser()).map(
        |(dest, ty, op, mut operands)| {
            HyInstr::FCmp(FCmp {
                dest,
                ty,
                lhs: operands.remove(0),
                rhs: operands.remove(0),
                op,
            })
        },
    );

    define_op!(binary, fadd, FAdd);
    define_op!(binary, fsub, FSub);
    define_op!(binary, fmul, FMul);
    define_op!(binary, fdiv, FDiv);
    define_op!(binary, frem, FRem);
    define_op!(unary, fneg, FNeg);

    let cloned_ctx = ctx.clone();
    let phi = instruction_dest_parser(ctx.named_name.clone())
        .padded()
        .then_ignore(just("phi"))
        .then_ignore(whitespace())
        .then(type_parser(ctx.registry).padded())
        .then(
            text::ident()
                .padded()
                .map(move |s: &str| (cloned_ctx.label_namer)(s.to_string()))
                .then_ignore(just(","))
                .then(
                    operand_parser(
                        ctx.func_retriver.clone(),
                        ctx.named_name.clone(),
                        ctx.label_namer.clone(),
                    )
                    .padded(),
                )
                .padded()
                .delimited_by(just("["), just("]"))
                .map(|(label, operand)| TplLabelOperand { label, operand })
                .padded()
                .separated_by(just(","))
                .collect::<SmallVec<TplLabelOperand, 4>>(),
        )
        .map(|((dest, ty), operands)| {
            HyInstr::Phi(Phi {
                dest,
                ty,
                values: operands.into_iter().map(|x| (x.operand, x.label)).collect(),
            })
        });

    let invoke = instruction_dest_parser(ctx.named_name.clone())
        .or_not()
        .padded()
        .then_ignore(just("invoke"))
        .then_ignore(whitespace())
        .then(cconv_parser().then_ignore(whitespace()).or_not())
        .then(maybe_type_parser(ctx.registry).then_ignore(whitespace()))
        .then(
            operand_parser(
                ctx.func_retriver.clone(),
                ctx.named_name.clone(),
                ctx.label_namer.clone(),
            )
            .padded(),
        )
        .then(
            operand_parser(
                ctx.func_retriver.clone(),
                ctx.named_name.clone(),
                ctx.label_namer.clone(),
            )
            .padded()
            .separated_by(just(","))
            .collect::<Vec<_>>()
            .delimited_by(just("("), just(")")),
        )
        .map(|((((dest, cconv), ty), function), operands)| {
            HyInstr::Invoke(Invoke {
                dest,
                cconv,
                ty,
                function,
                args: operands,
            })
        });

    let select = parse_simple_instruction(ctx.clone(), "select", 3, empty()).map(
        |(dest, ty, _, mut operands)| {
            HyInstr::Select(Select {
                dest,
                ty,
                condition: operands.remove(0),
                true_value: operands.remove(0),
                false_value: operands.remove(0),
            })
        },
    );

    let cast = parse_simple_instruction(ctx.clone(), "cast", 1, cast_op_parser()).map(
        |(dest, ty, op, mut operands)| {
            HyInstr::Cast(Cast {
                dest,
                op,
                source: operands.remove(0),
                ty,
            })
        },
    );

    let alignement_parser = just(",")
        .padded()
        .ignore_then(just("align"))
        .ignore_then(whitespace())
        .ignore_then(bigint_parser())
        .validate(|align, extra, emit| {
            if align.bits() <= 32 && align >= BigInt::from(1) {
                align.iter_u32_digits().next().unwrap() as u32
            } else {
                emit.emit(Rich::custom(
                    extra.span(),
                    format!(
                        "alignment must be a strictly positive integer fitting in 32 bits, got {}",
                        align
                    ),
                ));
                1u32
            }
        })
        .labelled("alignment");

    let ordering_parser = just(",")
        .padded()
        .ignore_then(just("atomic"))
        .ignore_then(whitespace())
        .ignore_then(ordering_parser())
        .labelled("memory ordering");

    let mem_postfix = choice((
        alignement_parser
            .clone()
            .or_not()
            .then(ordering_parser.clone().or_not()),
        ordering_parser
            .clone()
            .or_not()
            .then(alignement_parser.clone().or_not())
            .map(|(a, b)| (b, a)),
    ));

    let mload = parse_simple_instruction(ctx.clone(), "load", 1, just("volatile").or_not())
        .then(mem_postfix.clone())
        .map(
            |((dest, ty, is_volatile, mut operands), (alignement, ordering))| {
                HyInstr::MLoad(MLoad {
                    dest,
                    ty,
                    addr: operands.remove(0),
                    alignement,
                    ordering: ordering,
                    volatile: is_volatile.is_some(),
                })
            },
        );

    let mstore = just("store")
        .ignore_then(whitespace())
        .ignore_then(just("volatile").or_not().padded())
        .then(
            operand_parser(
                ctx.func_retriver.clone(),
                ctx.named_name.clone(),
                ctx.label_namer.clone(),
            )
            .padded()
            .separated_by(just(","))
            .exactly(2)
            .collect::<SmallVec<Operand, 2>>(),
        )
        .then(mem_postfix.clone())
        .map(|((is_volatile, mut operands), (alignement, ordering))| {
            HyInstr::MStore(MStore {
                addr: operands.remove(0),
                value: operands.remove(0),
                alignment: alignement,
                ordering,
                volatile: is_volatile.is_some(),
            })
        });

    let malloca = parse_simple_instruction(ctx.clone(), "alloca", 1, empty())
        .then(alignement_parser.or_not())
        .map(|((dest, ty, _, mut operands), alignment)| {
            HyInstr::MAlloca(MAlloca {
                dest,
                ty,
                count: operands.remove(0),
                alignment,
            })
        });

    let mgetelementptr = instruction_dest_parser(ctx.named_name.clone())
        .padded()
        .then_ignore(just("getelementptr"))
        .then_ignore(whitespace())
        .then(type_parser(ctx.registry).padded())
        .then(
            operand_parser(
                ctx.func_retriver.clone(),
                ctx.named_name.clone(),
                ctx.label_namer.clone(),
            )
            .padded()
            .separated_by(just(","))
            .at_least(1)
            .collect::<Vec<_>>(),
        )
        .map(|((dest, ty), mut indices)| {
            HyInstr::MGetElementPtr(MGetElementPtr {
                dest,
                ty,
                base: indices.remove(0),
                indices,
            })
        });

    choice((
        choice((
            iadd, isub, imul, idiv, irem, iand, ior, ixor, iimplies, iequiv, ineg, inot, icmp,
            isht, fcmp, fadd, fsub, fmul,
        )),
        choice((
            invoke,
            select,
            mload,
            mstore,
            malloca,
            cast,
            fdiv,
            frem,
            fneg,
            phi,
            mgetelementptr,
        )),
    ))
}

fn parse_terminator<'src>(
    func_retriver: impl Fn(String, FunctionPointerType) -> Option<Uuid> + Clone + 'src,
    named_name: impl Fn(String) -> Name + Clone + 'src,
    label_namer: impl Fn(String) -> Label + Clone + 'src,
) -> impl Parser<'src, &'src str, Terminator, extra::Err<Rich<'src, char>>> {
    let branch_parser = just("branch")
        .ignore_then(
            operand_parser(
                func_retriver.clone(),
                named_name.clone(),
                label_namer.clone(),
            )
            .padded(),
        )
        .then_ignore(just(",").padded())
        .then(label_parser(label_namer.clone()))
        .then_ignore(just(",").padded())
        .then(label_parser(label_namer.clone()))
        .map(|((cond, target_true), target_false)| {
            Terminator::CBranch(CBranch {
                cond,
                target_true,
                target_false,
            })
        })
        .labelled("branch terminator");

    let jump_parser = just("jump")
        .ignore_then(whitespace())
        .ignore_then(label_parser(label_namer.clone()))
        .map(|target| Terminator::Jump(Jump { target }))
        .labelled("jump terminator");

    let ret_parser = just("ret")
        .ignore_then(whitespace())
        .ignore_then(choice((
            just("void").to(None),
            operand_parser(func_retriver, named_name, label_namer.clone())
                .padded()
                .map(Some),
        )))
        .map(|value| Terminator::Ret(Ret { value }))
        .labelled("return terminator");

    choice((branch_parser, jump_parser, ret_parser)).labelled("terminator")
}

fn parse_block<'src>(
    func_retriver: impl Fn(String, FunctionPointerType) -> Option<Uuid> + Clone + 'src,
    named_name: impl Fn(String) -> Name + Clone + 'src,
    label_namer: impl Fn(String) -> Label + Clone + 'src,
    registry: &'src TypeRegistry,
) -> impl Parser<'src, &'src str, BasicBlock, extra::Err<Rich<'src, char>>> {
    let terminator_parser = parse_terminator(
        func_retriver.clone(),
        named_name.clone(),
        label_namer.clone(),
    );

    let ctx = CtxA {
        func_retriver,
        named_name,
        label_namer: label_namer.clone(),
        registry,
    };

    text::ident()
        .map(move |s: &str| label_namer(s.to_string()))
        .padded()
        .then_ignore(just(":"))
        .labelled("block label")
        .padded()
        .then(
            parse_instruction(ctx)
                .padded()
                .repeated()
                .collect::<Vec<_>>(),
        )
        .then(terminator_parser.padded())
        .padded()
        .map(|((label, instructions), terminator)| BasicBlock {
            label,
            instructions,
            terminator,
        })
        .labelled("block")
}

fn cconv_parser<'src>()
-> impl Parser<'src, &'src str, CallingConvention, extra::Err<Rich<'src, char>>> {
    choice((
        just("cc").to(CallingConvention::C),
        just("fastcc").to(CallingConvention::FastC),
        just("coldcc").to(CallingConvention::ColdC),
        just("ghccc").to(CallingConvention::GhcC),
        just("hipecc").to(CallingConvention::HipeC),
        just("anyregcc").to(CallingConvention::AnyRegC),
        just("preservemostcc").to(CallingConvention::PreserveMostC),
        just("preserveallcc").to(CallingConvention::PreserveAllC),
        just("preservenonecc").to(CallingConvention::PreserveNoneC),
        just("cxx_fast_tlscc").to(CallingConvention::CxxFastTlsC),
        just("tailcc").to(CallingConvention::TailC),
        just("swiftcc").to(CallingConvention::SwiftC),
        just("swifttailcc").to(CallingConvention::SwiftTailC),
        just("cfguard_checkcc").to(CallingConvention::CfguardCheckC),
        just("cc")
            .ignore_then(digits(10).to_slice().try_map(|digits, span| {
                let n: u32 = match u32::from_str_radix(digits, 10) {
                    Ok(num) => num,
                    Err(_) => {
                        return Err(Rich::custom(
                            span,
                            format!("invalid calling convention number: {}", digits),
                        ));
                    }
                };
                Ok(n)
            }))
            .map(|n| CallingConvention::Numbered(n)),
    ))
    .labelled("calling convention")
}

fn visibility_parser<'src>()
-> impl Parser<'src, &'src str, crate::modules::Visibility, extra::Err<Rich<'src, char>>> {
    choice((
        just("default")
            .or_not()
            .to(crate::modules::Visibility::Default),
        just("hidden").to(crate::modules::Visibility::Hidden),
        just("protected").to(crate::modules::Visibility::Protected),
    ))
    .labelled("visibility")
}

pub fn function_parser<'src>(
    func_retriver: impl Fn(String, FunctionPointerType) -> Option<Uuid> + Clone + 'src,
    registry: &'src TypeRegistry,
    uuid: Uuid,
) -> impl Parser<'src, &'src str, crate::modules::Function, extra::Err<Rich<'src, char>>> {
    let name_hashmap: Rc<RefCell<BTreeMap<String, Name>>> = Default::default();
    let named_name = move |string: String| {
        let hashmap = &mut *name_hashmap.borrow_mut();
        if let Some(name) = hashmap.get(&string) {
            name.clone()
        } else {
            let next_name = hashmap.len() as u32;
            hashmap.insert(string, next_name);
            next_name
        }
    };

    let label_hashmap: Rc<RefCell<BTreeMap<String, Label>>> = Default::default();
    let label_namer = move |string: String| {
        let hashmap = &mut *label_hashmap.borrow_mut();
        if let Some(label) = hashmap.get(&string) {
            label.clone()
        } else {
            let next_label = Label(hashmap.len() as u32);
            hashmap.insert(string, next_label);
            next_label
        }
    };

    just("define")
        .ignore_then(whitespace())
        .ignore_then(cconv_parser().then_ignore(whitespace()).or_not())
        .then(visibility_parser().then_ignore(whitespace()).or_not())
        .then(maybe_type_parser(registry))
        .then_ignore(whitespace())
        .then(percent_name_parser())
        .then(
            register_parser(named_name.clone())
                .then_ignore(just(":").padded())
                .then(type_parser(registry))
                .padded()
                .separated_by(just(","))
                .collect::<Vec<_>>()
                .delimited_by(just("("), just(")"))
                .padded(),
        )
        .then(
            parse_block(
                func_retriver.clone(),
                named_name.clone(),
                label_namer.clone(),
                registry,
            )
            // just("A")
            .padded()
            .repeated()
            .collect::<Vec<_>>()
            .delimited_by(just("{"), just("}"))
            .padded(),
        )
        .map(
            move |(((((cconv, visibility), return_type), func_name), params), blocks)| Function {
                uuid,
                name: Some(func_name.to_string()),
                params,
                return_type,
                // body: todo!(),
                body: blocks
                    .into_iter()
                    .map(|block| (block.label.clone(), block))
                    .collect(),
                visibility,
                cconv,
                wildcard_types: Default::default(),
                meta_function: false,
            },
        )
        .padded()
}

pub fn import_parser<'src>() -> impl Parser<'src, &'src str, String, extra::Err<Rich<'src, char>>> {
    just("import")
        .ignore_then(whitespace())
        .ignore_then(
            any()
                .filter(|c: &char| *c != '\n' && *c != '\r' && *c != '"' && *c != '\'')
                .repeated()
                .collect::<String>()
                .delimited_by(just("\""), just("\"")),
        )
        .map(|s: String| s.trim().to_string())
        .padded()
        .then_ignore(just(";"))
}

enum ModuleItem {
    Import(String),
    Function(Function),
}

pub fn extend_module_from_path(
    module: &mut Module,
    registry: &TypeRegistry,
    path: impl AsRef<Path>,
) -> Result<(), Error> {
    debug!("Extending module from path: {:?}", path.as_ref());
    let canonical_path = std::fs::canonicalize(&path).map_err(|e| Error::FileNotFound {
        path: path.as_ref().to_string_lossy().to_string(),
        cause: e,
    })?;

    /* Hashset of all absolute path that have been imported (avoid cyclic imports) */
    let mut imported_paths: HashSet<PathBuf> = Default::default();
    let mut queue: Vec<PathBuf> = vec![canonical_path.clone()];

    /* Construct index map for ext-func lookup */
    let ext_func_lookup: BTreeMap<String, Uuid> = module
        .functions
        .iter()
        .filter_map(|(uuid, func)| {
            if let Some(name) = &func.name {
                if func.visibility == Some(crate::modules::Visibility::Default) {
                    Some((name.clone(), *uuid))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    /* List of unresolved internal functions */
    let unresolved_internal_forward_map: RefCell<BTreeMap<Uuid, String>> = Default::default();
    let unresolved_internal_reverse_map: RefCell<BTreeMap<String, Uuid>> = Default::default();
    let unresolved_internal_forward_map_ref = &unresolved_internal_forward_map;
    let unresolved_internal_reverse_map_ref = &unresolved_internal_reverse_map;
    let mut resolve_map: BTreeMap<Uuid, Uuid> = Default::default();

    /* Function retriever */
    let func_retriver = {
        let module = module.clone();
        move |name: String, kind: FunctionPointerType| -> Option<Uuid> {
            match kind {
                FunctionPointerType::Internal => {
                    /* Search in the current module */
                    for (uuid, func) in &module.functions {
                        if let Some(func_name) = &func.name {
                            if *func_name == name {
                                return Some(*uuid);
                            }
                        }
                    }

                    /* If not found, register as unresolved internal function */
                    let mut unresolved_internal_forward_map =
                        unresolved_internal_forward_map_ref.borrow_mut();
                    let mut unresolved_internal_reverse_map =
                        unresolved_internal_reverse_map_ref.borrow_mut();
                    if let Some(uuid) = unresolved_internal_reverse_map.get(&name) {
                        Some(*uuid)
                    } else {
                        let new_uuid = Uuid::new_v4();
                        unresolved_internal_forward_map.insert(new_uuid, name.clone());
                        unresolved_internal_reverse_map.insert(name, new_uuid);
                        Some(new_uuid)
                    }
                }
                FunctionPointerType::External => ext_func_lookup.get(&name).copied(),
            }
        }
    };

    /* Main loop to process the import queue */
    while let Some(current_path) = queue.pop() {
        if imported_paths.contains(&current_path) {
            continue;
        }
        imported_paths.insert(current_path.clone());

        /* Read the file content */
        debug!("Reading module from path: {:?}", current_path);
        let content = std::fs::read_to_string(&current_path).map_err(|e| Error::FileNotFound {
            path: path.as_ref().to_string_lossy().to_string(),
            cause: e,
        })?;

        /* Build the 'main' parser */
        let file_parser = choice((
            import_parser().map(ModuleItem::Import),
            function_parser(func_retriver.clone(), registry, Uuid::new_v4())
                .map(ModuleItem::Function),
        ))
        .padded()
        .repeated()
        .collect::<Vec<_>>();

        /* Parse the file content */
        let parse_result = file_parser.parse(&content);
        if parse_result.has_errors() {
            let errors = parse_result
                .errors()
                .map(|error| {
                    log::error!(
                        "Error parsing file {}: {}",
                        current_path.to_string_lossy(),
                        error
                    );

                    ParserError {
                        file: current_path.to_string_lossy().to_string(),
                        start: error.span().start(),
                        end: error.span().end(),
                        message: error.reason().to_string(),
                    }
                })
                .collect();
            return Err(Error::ParserErrors { errors });
        }

        if let Some(output) = parse_result.into_output() {
            for item in output {
                match item {
                    ModuleItem::Import(import) => {
                        /* If this is a relative path, make it relative to the current file
                        parent directory */
                        let import_path = PathBuf::from(import);
                        let import_path = if import_path.is_relative() {
                            let parent = current_path.parent().unwrap();
                            parent.join(import_path)
                        } else {
                            import_path
                        };

                        let canonical_import_path =
                            std::fs::canonicalize(&import_path).map_err(|e| {
                                Error::FileNotFound {
                                    path: import_path.to_string_lossy().to_string(),
                                    cause: e,
                                }
                            })?;

                        /* Add the import path to the queue */
                        queue.push(canonical_import_path);
                    }
                    ModuleItem::Function(function) => {
                        /* Check if this function was an unresolved internal function */
                        let unresolved_internal_reverse_map =
                            unresolved_internal_reverse_map_ref.borrow();
                        if let Some(name) = &function.name {
                            if let Some(&temp_uuid) = unresolved_internal_reverse_map.get(name) {
                                /* Register the function under the original UUID */
                                if resolve_map.insert(temp_uuid, function.uuid).is_some() {
                                    return Err(Error::DuplicateFunctionName {
                                        name: name.clone(),
                                        file: current_path.to_string_lossy().to_string(),
                                    });
                                }
                            }
                        }

                        module.functions.insert(function.uuid, function.clone());
                    }
                }
            }
        }
    }

    /* Verify all internal reference have been resolved */
    let unresolved_internal_forward_map = unresolved_internal_forward_map_ref.borrow();
    if resolve_map.len() < unresolved_internal_forward_map.len() {
        /* Find all unresolved internal functions */
        let unresolved: Vec<String> = unresolved_internal_forward_map
            .iter()
            .filter_map(|(uuid, name)| {
                if !resolve_map.contains_key(uuid) {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect();

        /* Debug the error */
        for name in &unresolved {
            log::error!("Unresolved internal function: {}", name);
        }
        return Err(Error::UnresolvedInternalFunctions { names: unresolved });
    }

    /* Apply the resolve map to all functions */
    for (_, function) in &mut module.functions {
        for (_, block) in &mut function.body {
            for instr in &mut block.instructions {
                for operand in instr.operands_mut() {
                    match operand {
                        Operand::Imm(AnyConst::FuncPtr(FunctionPointer::Internal(
                            internal_ptr,
                        ))) => {
                            if let Some(new_uuid) = resolve_map.get(&internal_ptr) {
                                *internal_ptr = *new_uuid;
                            }
                        }
                        _ => {}
                    }
                }
            }

            for operand in block.terminator.operands_mut() {
                match operand {
                    Operand::Imm(AnyConst::FuncPtr(FunctionPointer::Internal(internal_ptr))) => {
                        if let Some(new_uuid) = resolve_map.get(&internal_ptr) {
                            *internal_ptr = *new_uuid;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

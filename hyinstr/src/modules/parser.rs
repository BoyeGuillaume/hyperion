use std::{cell::RefCell, collections::BTreeMap, rc::Rc, u16, u64};

use bigdecimal::{BigDecimal, Num};
use chumsky::{prelude::*, text::digits};
use either::Either;
use num_bigint::BigInt;
use smallvec::SmallVec;
use uuid::Uuid;

use crate::{
    consts::{fp::FConst, int::IConst},
    modules::{
        BasicBlock, CallingConvention, Function,
        instructions::HyInstr,
        int::*,
        operand::{Label, Name, Operand},
        symbol::{FunctionPointer, FunctionPointerType},
        terminator::{CBranch, Jump, Ret, Terminator},
    },
    types::{
        AnyType, TypeRegistry, Typeref,
        aggregate::{ArrayType, StructType},
        primary::{FType, IType, PrimaryBasicType, PtrType, VcSize, VcType},
    },
};

pub fn whitespace<'src>() -> impl Parser<'src, &'src str, (), extra::Err<Rich<'src, char>>> + Clone
{
    any()
        .filter(|c: &char| c.is_whitespace())
        .repeated()
        .at_least(1)
        .ignored()
        .labelled("whitespace")
}

pub fn itype_parser<'src>()
-> impl Parser<'src, &'src str, IType, extra::Err<Rich<'src, char>>> + Clone {
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

pub fn ftype_parser<'src>()
-> impl Parser<'src, &'src str, FType, extra::Err<Rich<'src, char>>> + Clone {
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

pub fn tptr_parser<'src>()
-> impl Parser<'src, &'src str, PtrType, extra::Err<Rich<'src, char>>> + Clone {
    just("ptr").to(PtrType).labelled("function pointer type")
}

pub fn bigint_parser<'src>()
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

pub fn primary_type_parser<'src>()
-> impl Parser<'src, &'src str, PrimaryBasicType, extra::Err<Rich<'src, char>>> + Clone {
    choice((
        itype_parser().map(PrimaryBasicType::Int),
        ftype_parser().map(PrimaryBasicType::Float),
        tptr_parser().map(PrimaryBasicType::Ptr),
    ))
    .labelled("primitive type")
}

pub fn type_parser<'src>(
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

pub fn uuid_parser<'src>() -> impl Parser<'src, &'src str, uuid::Uuid, extra::Err<Rich<'src, char>>>
{
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

pub fn iconst_parser<'src>() -> impl Parser<'src, &'src str, IConst, extra::Err<Rich<'src, char>>> {
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

pub fn decimal_query<'src>()
-> impl Parser<'src, &'src str, BigDecimal, extra::Err<Rich<'src, char>>> {
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

pub fn fp_parser<'src>() -> impl Parser<'src, &'src str, FConst, extra::Err<Rich<'src, char>>> {
    ftype_parser()
        .then_ignore(whitespace())
        .then(decimal_query())
        .map(|(ty, value)| FConst { ty, value })
        .labelled("floating-point constant")
}

pub fn func_ptr_parser<'src>(
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

pub fn label_parser<'src>(
    named_label: impl Fn(String) -> Label + Clone + 'src,
) -> impl Parser<'src, &'src str, Label, extra::Err<Rich<'src, char>>> + Clone {
    just("label")
        .then_ignore(whitespace())
        .ignore_then(chumsky::text::ascii::ident())
        .map(move |s: &str| named_label(s.to_string()))
        .labelled("label")
}

pub fn percent_name_parser<'src>()
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

pub fn register_parser<'src>(
    named_name: impl Fn(String) -> Name + 'src,
) -> impl Parser<'src, &'src str, Name, extra::Err<Rich<'src, char>>> {
    percent_name_parser()
        .labelled("register")
        .map(move |x| named_name(x))
}

pub fn operand_parser<'src>(
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

impl<const N: usize> chumsky::container::Container<Operand> for SmallVec<Operand, N> {
    fn with_capacity(n: usize) -> Self {
        SmallVec::with_capacity(n)
    }

    fn push(&mut self, item: Operand) {
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

fn parse_simple_arith<'src, U, F1, F2, F3>(
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
        .then_ignore(just(opname).padded())
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
        just("nonwarp").to(OverflowPolicy::Panic),
        just("wrap").to(OverflowPolicy::Wrap),
        just("saturate").to(OverflowPolicy::Saturate),
    ))
    .labelled("overflow policy");

    let integer_signedness_parser = choice((
        just("signed").to(IntegerSignedness::Signed),
        just("unsigned").to(IntegerSignedness::Unsigned),
    ))
    .labelled("integer signedness");

    macro_rules! define_i_binop {
        (
            $opname:ident,
            $actual:ident
        ) => {
            let $opname = parse_simple_arith(
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
            nopolicy
            $opname:ident,
            $actual:ident
        ) => {
            let $opname = parse_simple_arith(
                ctx.clone(),
                stringify!($opname),
                2,
                integer_signedness_parser.padded(),
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
            simple
            $opname:ident,
            $actual:ident
        ) => {
            let $opname = parse_simple_arith(ctx.clone(), stringify!($opname), 2, empty()).map(
                |(dest, ty, _, mut operands)| {
                    HyInstr::$actual($actual {
                        dest,
                        ty,
                        lhs: operands.remove(0),
                        rhs: operands.remove(0),
                    })
                },
            );
        };
    }

    define_i_binop!(iadd, IAdd);
    define_i_binop!(isub, ISub);
    define_i_binop!(imul, IMul);
    define_i_binop!(nopolicy idiv, IDiv);
    define_i_binop!(nopolicy irem, IRem);
    define_i_binop!(simple iand, IAnd);
    define_i_binop!(simple ior, IOr);
    define_i_binop!(simple ixor, IXor);
    define_i_binop!(simple iimplies, IImplies);
    define_i_binop!(simple iequiv, IEquiv);

    // let iadd = parse_simple_arith(ctx, "iadd", 2, integer_policy_parser.clone()).map(
    //     |(dest, ty, (overflow_policy, signess), mut operands)| {
    //         HyInstr::IAdd(IAdd {
    //             dest,
    //             ty,
    //             lhs: operands.pop().unwrap(),
    //             rhs: operands.pop().unwrap(),
    //             overflow: overflow_policy,
    //             signedness: signess,
    //         })
    //     },
    // );
    // // general format: (%<dest> = )?<opcode> <keywords> <ty> <operands>
    // exceptions:
    //  phi: %<dest> = phi <ty> [<value>, <label>], [<value>, <label>], ...
    //  memory instructions: has suffixes like , align <num>, volatile, etc.
    // let dest_parser = just("%")
    //     .ignore_then(
    //         any()
    //             .filter(|c: &char| c.is_ascii_alphanumeric() || *c == '_')
    //             .repeated()
    //             .collect::<String>()
    //             .labelled("destination register"),
    //     )
    //     .padded()
    //     .then_ignore(just('='))
    //     .padded()
    //     .labelled("destination");

    // let opcode = any()
    //     .filter(|c: &char| c.is_ascii_alphabetic())
    //     .repeated()
    //     .to_slice()
    //     .validate(|s: &str, extra, emit| match HyInstrOp::from_string(s) {
    //         Some(op) => op,
    //         None => {
    //             emit.emit(Rich::custom(
    //                 extra.span(),
    //                 format!(
    //                     "unknown opcode: {} (expected one of: {})",
    //                     s,
    //                     HyInstrOp::iter()
    //                         .map(|x| x.opname())
    //                         .collect::<Vec<_>>()
    //                         .join(", ")
    //                 ),
    //             ));
    //             HyInstrOp::IAdd
    //         }
    //     })
    //     .labelled("opcode");

    choice((
        iadd, isub, imul, idiv, irem, iand, ior, ixor, iimplies, iequiv,
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

pub fn parse_block<'src>(
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

pub fn cconv_parser<'src>()
-> impl Parser<'src, &'src str, CallingConvention, extra::Err<Rich<'src, char>>> {
    // CallingConvention::HipeC => "hipecc".into(),
    // CallingConvention::AnyRegC => "anyregcc".into(),
    // CallingConvention::PreserveMostC => "preservemostcc".into(),
    // CallingConvention::PreserveAllC => "preserveallcc".into(),
    // CallingConvention::PreserveNoneC => "preservenonecc".into(),
    // CallingConvention::CxxFastTlsC => "cxx_fast_tlscc".into(),
    // CallingConvention::TailC => "tailcc".into(),
    // CallingConvention::SwiftC => "swiftcc".into(),
    // CallingConvention::SwiftTailC => "swifttailcc".into(),
    // CallingConvention::CfguardCheckC => "cfguard_checkcc".into(),
    // CallingConvention::Numbered(n) => format!("cc{}", n).into(),
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

pub fn visibility_parser<'src>()
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
    next_func_uuid: impl Fn() -> Uuid + 'src,
) -> impl Parser<'src, &'src str, crate::modules::Function, extra::Err<Rich<'src, char>>> {
    let maybe_type_parser = choice((
        type_parser(registry).map(Option::Some),
        just("void").to(None),
    ))
    .labelled("maybe type");

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
        .ignore_then(maybe_type_parser)
        .then_ignore(whitespace())
        .then(cconv_parser().then_ignore(whitespace()).or_not())
        .then(visibility_parser().then_ignore(whitespace()).or_not())
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
            move |(((((return_type, cconv), visibility), func_name), params), blocks)| Function {
                uuid: next_func_uuid(),
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

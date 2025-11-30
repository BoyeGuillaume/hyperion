use std::u64;

use bigdecimal::{BigDecimal, Num};
use chumsky::{
    IterParser, Parser,
    error::Rich,
    extra,
    prelude::{any, choice, just},
    text::{self, Padded, digits},
};
use num_bigint::BigInt;

use crate::{
    consts::{fp::FConst, int::IConst},
    modules::operand::{Label, Name, Operand},
    types::primary::{FType, IType, PtrType},
};

pub fn itype_parser<'src>() -> impl Parser<'src, &'src str, IType, extra::Err<Rich<'src, char>>> {
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

pub fn ftype_parser<'src>() -> impl Parser<'src, &'src str, FType, extra::Err<Rich<'src, char>>> {
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

pub fn ptr_parser<'src>() -> impl Parser<'src, &'src str, PtrType, extra::Err<Rich<'src, char>>> {
    just("ptr").to(PtrType).labelled("function pointer type")
}

pub fn number_parser<'src>() -> impl Parser<'src, &'src str, BigInt, extra::Err<Rich<'src, char>>> {
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

pub fn iconst_parser<'src>() -> impl Parser<'src, &'src str, IConst, extra::Err<Rich<'src, char>>> {
    itype_parser()
        .then_ignore(
            any()
                .filter(|s: &char| s.is_whitespace())
                .repeated()
                .at_least(1)
                .labelled("whitespace"),
        )
        .then(number_parser())
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
        .then_ignore(
            any()
                .filter(|s: &char| s.is_whitespace())
                .repeated()
                .at_least(1)
                .labelled("whitespace"),
        )
        .then(decimal_query())
        .map(|(ty, value)| FConst { ty, value })
        .labelled("floating-point constant")
}

pub fn parse_operand<'src>(
    named_name: impl Fn(String) -> Name,
    named_label: impl Fn(String) -> Label,
) -> impl Parser<'src, &'src str, Operand, extra::Err<Rich<'src, char>>> {
    let reg_parser = just("%")
        .ignore_then(
            any()
                .filter(|c: &char| c.is_ascii_alphabetic() || *c == '_')
                .repeated()
                .collect::<String>()
                .labelled("identifier"),
        )
        .labelled("register")
        .map(move |x| Operand::Reg(named_name(x)));

    let imm_parser = choice((
        iconst_parser().map(|x| Operand::Imm(x.into())),
        fp_parser().map(|x| Operand::Imm(x.into())),
    ))
    .labelled("immediate");

    let lbl_parser = just("label")
        .then_ignore(
            any()
                .filter(|s: &char| s.is_whitespace())
                .repeated()
                .at_least(1)
                .labelled("whitespace"),
        )
        .ignore_then(chumsky::text::ascii::ident())
        .map(move |s: &str| named_label(s.to_string()))
        .map(Operand::Lbl)
        .labelled("label");

    choice((reg_parser, imm_parser, lbl_parser))
}

// fn parse_instruction<'src>(
//     input: &'src str,
// ) -> impl Parser<'src, &'src str, HyInstr, extra::Err<Rich<'src, char>>> {
//     // general format: (%<dest> = )?<opcode> <keywords> <ty> <operands>
//     // exceptions:
//     //  phi: %<dest> = phi <ty> [<value>, <label>], [<value>, <label>], ...
//     //  memory instructions: has suffixes like , align <num>, volatile, etc.
//     todo!()
// }

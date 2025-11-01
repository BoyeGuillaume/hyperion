#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use strum::{EnumIs, EnumTryAs};
use uuid::Uuid;

/// Represents an integer type with a specific bit width.
///
/// Signeness is not represented here; all integer types are treated as unsigned.
/// Instructions that operate on signed integers will interpret the bits accordingly.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", Serialize, Deserialize)]
#[repr(transparent)]
pub struct IType {
    num_bits: u32,
}

impl IType {
    /// Common integer types used in Hy.
    pub const I1: Self = Self { num_bits: 1 };
    pub const I8: Self = Self { num_bits: 8 };
    pub const I16: Self = Self { num_bits: 16 };
    pub const I32: Self = Self { num_bits: 32 };
    pub const I64: Self = Self { num_bits: 64 };
    pub const I128: Self = Self { num_bits: 128 };
    pub const MIN_BITS: u32 = 1;
    pub const MAX_BITS: u32 = (1 << 23) - 1;

    #[inline]
    const fn check_validity(num_bits: u32) -> bool {
        num_bits >= 1 && num_bits < (1 << 23)
    }

    /// Creates a new `IntType` with the specified number of bits.
    #[inline]
    pub const fn new(num_bits: u32) -> Option<Self> {
        if Self::check_validity(num_bits) {
            Some(Self { num_bits })
        } else {
            None
        }
    }

    /// Returns the number of bits of the integer type.
    #[inline]
    pub const fn num_bits(&self) -> u32 {
        self.num_bits
    }

    /// Returns the number of bytes required to store the integer type.
    #[inline]
    pub const fn byte_size(&self) -> u32 {
        (self.num_bits + 7) / 8
    }

    /// Returns `true` if the integer type is byte-aligned (i.e., its number of bits is a multiple of 8).
    #[inline]
    pub const fn byte_aligned(&self) -> bool {
        self.num_bits % 8 == 0
    }

    /// Returns the maximum value that can be represented by this integer type.
    ///
    /// Notice that this maximum value is itself limited to u64, for bigger integers
    /// we simply return `None`.
    #[inline]
    pub const fn max_value(&self) -> Option<u64> {
        if self.num_bits > 64 {
            None
        } else if self.num_bits == 64 {
            Some(u64::MAX)
        } else {
            Some((1u64 << self.num_bits) - 1)
        }
    }
}

impl std::fmt::Display for IType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "i{}", self.num_bits)
    }
}

/// Represents a floating-point type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", Serialize, Deserialize)]
pub enum FType {
    /// 16-bit floating point value (IEEE-754 binary16)
    /// Also known as "half precision".
    Fp16,

    /// 16-bit "brain" floating point value (7-bit significand). Provide
    /// the same number of exponent bits as `FType::Fp32`, so that it matches the
    /// dynamic range but with greatly reduced precision. Used in Intel's
    /// AVX-512 BF16 extensions and ARM's ARMv8.6-A extensions.
    Bf16,

    /// 32-bit floating point value (IEEE-754 binary32)
    /// Also known as "single precision".
    /// Corresponds to Rust's `f32` type.
    Fp32,

    /// 64-bit floating point value (IEEE-754 binary64)
    /// Also known as "double precision".
    /// Corresponds to Rust's `f64` type.
    Fp64,

    /// 128-bit floating point value (IEEE-754 binary128)
    /// Also known as "quadruple precision".
    Fp128,

    /// 80-bit floating point value (X87 extended precision)
    /// Mainly used in x86 architectures.
    X86Fp80,

    /// 128-bit floating point value (two 64-bit values)
    PPCFp128,
}

impl std::fmt::Display for FType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            let s = match self {
                FType::Fp16 => "fp16",
                FType::Bf16 => "bf16",
                FType::Fp32 => "fp32",
                FType::Fp64 => "fp64",
                FType::Fp128 => "fp128",
                FType::X86Fp80 => "x86_fp80",
                FType::PPCFp128 => "ppc_fp128",
            };
            write!(f, "{}", s)
        } else {
            let s = match self {
                FType::Fp16 => "half",
                FType::Bf16 => "bfloat",
                FType::Fp32 => "float",
                FType::Fp64 => "double",
                FType::Fp128 => "fp128",
                FType::X86Fp80 => "x86_fp80",
                FType::PPCFp128 => "ppc_fp128",
            };
            write!(f, "{}", s)
        }
    }
}

/// Target extension types represent types that must be preserved through optimization,
/// but are otherwise generally opaque to the compiler.
///
/// They may be used as function parameters or arguments, and in phi or select instructions.
/// Some types may be also used in alloca instructions or as global values, and correspondingly
/// it is legal to use load and store instructions on them. Full semantics for these types are
/// defined by the target.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", Serialize, Deserialize)]
pub struct ExtType {
    pub ext: Uuid,
    pub parameters: Box<[u32]>,
}

impl std::fmt::Display for ExtType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.parameters.is_empty() {
            write!(f, "ext<{}>", self.ext)
        } else {
            write!(
                f,
                "ext<{}>({})",
                self.ext,
                self.parameters
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }
}

/// Pointer type is represented as a primary basic type.
///
/// The pointer type ptr is used to specify memory locations. Pointers are commonly used to reference objects in memory. By default
/// pointers are opaque and do not have an associated pointee type. Pointer arithmetic and dereferencing requires to add type information
/// to ensure behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", Serialize, Deserialize)]
pub struct PtrType;

impl std::fmt::Display for PtrType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ptr")
    }
}

/// Primary base types used for vector types and other constructs.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, EnumTryAs, EnumIs)]
#[cfg_attr(feature = "serde", Serialize, Deserialize)]
pub enum PrimaryBasicType {
    Int(IType),
    Float(FType),
    Ext(ExtType),
    Ptr(PtrType),
}

impl From<IType> for PrimaryBasicType {
    fn from(itype: IType) -> Self {
        PrimaryBasicType::Int(itype)
    }
}

impl From<FType> for PrimaryBasicType {
    fn from(ftype: FType) -> Self {
        PrimaryBasicType::Float(ftype)
    }
}

impl From<ExtType> for PrimaryBasicType {
    fn from(exttype: ExtType) -> Self {
        PrimaryBasicType::Ext(exttype)
    }
}

impl From<PtrType> for PrimaryBasicType {
    fn from(ptrtype: PtrType) -> Self {
        PrimaryBasicType::Ptr(ptrtype)
    }
}

impl std::fmt::Display for PrimaryBasicType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrimaryBasicType::Int(itype) => write!(f, "{}", itype),
            PrimaryBasicType::Float(ftype) => write!(f, "{}", ftype),
            PrimaryBasicType::Ext(exttype) => write!(f, "{}", exttype),
            PrimaryBasicType::Ptr(ptrtype) => write!(f, "{}", ptrtype),
        }
    }
}

/// Size of a vector type, either fixed or scalable.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", Serialize, Deserialize)]
pub enum VectorSize {
    /// Fixed size vector with the given number of elements.
    Fixed(u16),

    /// Scalable size vector where number of elements is a multiple of the given factor.
    Scalable(u16),
}

/// A vector type is a simple derived type that represents a vector of elements.
///
/// Vector types are used when multiple primitive data are operated in parallel using a single instruction (SIMD).
/// A vector type requires a size (number of elements), an underlying primitive data type, and a scalable property
/// to represent vectors where the exact hardware vector length is unknown at compile time.
///
/// Vector types are considered primary types.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", Serialize, Deserialize)]
pub struct VcType {
    pub ty: PrimaryBasicType,
    pub size: VectorSize,
}

impl std::fmt::Display for VcType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.size {
            VectorSize::Fixed(num) => write!(f, "<{} x {}>", num, self.ty),
            VectorSize::Scalable(num) => write!(f, "<vscale {} x {}>", num, self.ty),
        }
    }
}

/// The label type represents code labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", Serialize, Deserialize)]
pub struct LblType;

impl std::fmt::Display for LblType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "label")
    }
}

/// Represents any primitive type.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PrimaryType {
    Int(IType),
    Float(FType),
    Ext(ExtType),
    Ptr(PtrType),
    Vc(VcType),
    Lbl(LblType),
}

macro_rules! primary_type_from {
    ($typ:ty, $lbl:ident) => {
        impl From<$typ> for PrimaryType {
            fn from(value: $typ) -> Self {
                PrimaryType::$lbl(value)
            }
        }
    };
}

primary_type_from! { IType, Int }
primary_type_from! { FType, Float }
primary_type_from! { ExtType, Ext }
primary_type_from! { PtrType, Ptr }
primary_type_from! { VcType, Vc }
primary_type_from! { LblType, Lbl }

impl std::fmt::Display for PrimaryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrimaryType::Int(itype) => itype.fmt(f),
            PrimaryType::Float(ftype) => ftype.fmt(f),
            PrimaryType::Ext(ext_type) => ext_type.fmt(f),
            PrimaryType::Ptr(ptr_type) => ptr_type.fmt(f),
            PrimaryType::Vc(vc_type) => vc_type.fmt(f),
            PrimaryType::Lbl(lbl_type) => lbl_type.fmt(f),
        }
    }
}

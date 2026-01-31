//! Primary types
//!
//! This file contains the primitive and vector types used by the instruction
//! representation. Primary types are the building blocks for more complex types
//! (for example vectors, arrays and structures). They are intentionally small
//! and copyable/value-like so they can be embedded inside other descriptors.
//!
//! The important types are:
//! - `IType`: an integer width in bits (1..<(1<<23)).
//! - `FType`: floating point kinds (fp16, fp32, fp64, etc.).
//! - `ExtType`: opaque target-specific types.
//! - `PtrType`: opaque pointer type.
//! - `VcType`: vector type (element type + size).
//! - `LblType`: label type used for code labels.
//!
//! These types implement `Display` for readable formatting and can be
//! serialized with the optional `serde` feature.
use num_bigint::BigInt;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use strum::{EnumIs, EnumIter, EnumTryAs, IntoEnumIterator};
use uuid::Uuid;

/// Represents an integer type with a specific bit width.
///
/// Signeness is not represented here; all integer types are treated as unsigned.
/// Instructions that operate on signed integers will interpret the bits accordingly.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
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
    pub const fn try_new(num_bits: u32) -> Option<Self> {
        if Self::check_validity(num_bits) {
            Some(Self { num_bits })
        } else {
            None
        }
    }

    /// Same as `try_new` but panics if the number of bits is invalid.
    #[inline]
    pub fn new(num_bits: u32) -> Self {
        match Self::try_new(num_bits) {
            Some(itype) => itype,
            None => panic!("invalid integer type width: {}", num_bits),
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
        self.num_bits.div_ceil(8)
    }

    /// Returns `true` if the integer type is byte-aligned (i.e., its number of bits is a multiple of 8).
    #[inline]
    pub const fn byte_aligned(&self) -> bool {
        self.num_bits.is_multiple_of(8)
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

    /// Check whether a given [`num_bigint::BigInt`] value fits in this integer type.
    pub fn fits_value(&self, value: &BigInt) -> bool {
        let bits = value.bits();
        bits <= self.num_bits as u64
    }
}

impl std::fmt::Display for IType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "i{}", self.num_bits)
    }
}

/// Represents a floating-point type.
///
/// Different floating-point types correspond to different precisions and
/// formats. Not all floating-point types may be supported on all targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, EnumIter)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
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

impl std::str::FromStr for FType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        FType::iter().find(|ftype| ftype.to_str() == s).ok_or(())
    }
}

impl FType {
    /// Returns the string representation of the floating-point type.
    pub fn to_str(&self) -> &'static str {
        match self {
            FType::Fp16 => "fp16",
            FType::Bf16 => "bf16",
            FType::Fp32 => "fp32",
            FType::Fp64 => "fp64",
            FType::Fp128 => "fp128",
            FType::X86Fp80 => "x86_fp80",
            FType::PPCFp128 => "ppc_fp128",
        }
    }

    /// Returns the number of bytes required to store the floating-point type.
    #[inline]
    pub fn byte_size(&self) -> u32 {
        match self {
            FType::Fp16 | FType::Bf16 => 2,
            FType::Fp32 => 4,
            FType::Fp64 => 8,
            FType::Fp128 | FType::PPCFp128 => 16,
            FType::X86Fp80 => 10,
        }
    }
}

impl std::fmt::Display for FType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_str())
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
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct ExtType {
    /// Unique identifier describing the external type class.
    pub ext: Uuid,
    /// Target defined parameters carried by the type.
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

/// Represents a wildcard type used in generic instructions
///
/// Any function containing wildcard are considered incomplete and cannot be
/// used until all wildcards are resolved to concrete types. Incomplete functions
/// may reference other incomplete functions which must be fully resolved in the context
/// of the caller.
///
/// The wildcard type is identified by a unique ID to allow multiple wildcards
/// to coexist in the same function or module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct WType {
    /// Ordinal identifier used to distinguish wildcard placeholders.
    pub id: u16,
}

impl std::fmt::Display for WType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.id <= 25 {
            let c = (b'A' + (self.id as u8)) as char;
            write!(f, "{}", c)
        } else {
            write!(f, "_{}", self.id)
        }
    }
}

/// Pointer type is represented as a primary basic type.
///
/// The pointer type ptr is used to specify memory locations. Pointers are commonly used to reference objects in memory. By default
/// pointers are opaque and do not have an associated pointee type. Pointer arithmetic and dereferencing requires to add type information
/// to ensure behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct PtrType;

impl std::fmt::Display for PtrType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ptr")
    }
}

/// Primary base types used for vector types and other constructs.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, EnumTryAs, EnumIs)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub enum PrimaryBasicType {
    Int(IType),
    Float(FType),
    Ext(ExtType),
    Ptr(PtrType),
    Wildcard(WType),
}

impl From<PrimaryBasicType> for PrimaryType {
    fn from(pbt: PrimaryBasicType) -> Self {
        match pbt {
            PrimaryBasicType::Int(itype) => PrimaryType::Int(itype),
            PrimaryBasicType::Float(ftype) => PrimaryType::Float(ftype),
            PrimaryBasicType::Ext(exttype) => PrimaryType::Ext(exttype),
            PrimaryBasicType::Ptr(ptrtype) => PrimaryType::Ptr(ptrtype),
            PrimaryBasicType::Wildcard(wtype) => PrimaryType::Wildcard(wtype),
        }
    }
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

impl From<WType> for PrimaryBasicType {
    fn from(wtype: WType) -> Self {
        PrimaryBasicType::Wildcard(wtype)
    }
}

impl std::fmt::Display for PrimaryBasicType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrimaryBasicType::Int(itype) => write!(f, "{}", itype),
            PrimaryBasicType::Float(ftype) => write!(f, "{}", ftype),
            PrimaryBasicType::Ext(exttype) => write!(f, "{}", exttype),
            PrimaryBasicType::Ptr(ptrtype) => write!(f, "{}", ptrtype),
            PrimaryBasicType::Wildcard(wtype) => write!(f, "{}", wtype),
        }
    }
}

/// Size of a vector type, either fixed or scalable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub enum VcSize {
    /// Fixed size vector with the given number of elements.
    ///
    /// The number of elements must be greater than zero, and is known at compile time.
    Fixed(u16),

    /// Scalable size vector where number of elements is a multiple of the given factor.
    ///
    /// This is typically used to represent "scalable SIMD" such as
    /// - ARM SVE vectors (ARMv8.2-A and later)
    Scalable(u16),
}

/// A vector type represents multiple elements of a primitive type grouped for
/// parallel operations (SIMD).
///
/// Vector types combine an element type with a size, which can be either a fixed
/// number of lanes or a scalable multiple of the hardware "vscale" factor used
/// by scalable-SIMD ISAs (for example ARM SVE). You can see [`VcSize`] for
/// further information. They model the logical SIMD lanes used by vector
/// instructions; exact runtime lane counts for scalable vectors depend on the
/// target's runtime factor.
///
/// The semantics of a vector (alignment, element byte-size, etc.) are derived
/// from its element [`PrimaryBasicType`] and the [`VcSize`]. Vector types are
/// considered primary types and can be embedded inside other type descriptors.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct VcType {
    /// Element type stored in the vector lanes.
    pub ty: PrimaryBasicType,
    /// Vector lane descriptor (fixed or scalable).
    pub size: VcSize,
}

impl std::fmt::Display for VcSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VcSize::Fixed(num) => write!(f, "{}", num),
            VcSize::Scalable(num) => write!(f, "vscale {}", num),
        }
    }
}

impl std::fmt::Display for VcType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<{} x {}>", self.size, self.ty)
    }
}

impl VcType {
    /// Consutructs a new fixed-size vector type.
    pub fn fixed(ty: impl Into<PrimaryBasicType>, num: u16) -> Self {
        VcType {
            ty: ty.into(),
            size: VcSize::Fixed(num),
        }
    }
}

impl From<usize> for VcSize {
    fn from(num: usize) -> Self {
        VcSize::Fixed(num as u16)
    }
}

/// The label type represents code labels.
///
/// Labels are used as targets for control flow instructions like branches and jumps.
/// You can check [`crate::modules::operand::Label`] for more information. An added
/// constraint in hyperion is that labels should not cross function boundaries; they
/// are local to a function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub struct LblType;

impl std::fmt::Display for LblType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "label")
    }
}

/// Represents any primitive type used by the IR.
///
/// This sum type wraps concrete primary kinds like integers, floats, opaque
/// extension types, pointers, vectors and labels.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, EnumIs, EnumTryAs)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "borsh",
    derive(borsh::BorshSerialize, borsh::BorshDeserialize)
)]
pub enum PrimaryType {
    /// Integer type
    ///
    /// See [`IType`] for details.
    Int(IType),

    /// Floating point type
    ///
    /// See [`FType`] for details.
    Float(FType),

    /// Extension type
    ///
    /// See [`ExtType`] for details.
    Ext(ExtType),

    /// Pointer type
    ///
    /// See [`PtrType`] for details.
    Ptr(PtrType),

    /// Wildcard type
    ///
    /// See [`WType`] for details.
    Wildcard(WType),

    /// Vector type
    ///
    /// See [`VcType`] for details.
    Vc(VcType),

    /// Label type
    ///
    /// See [`LblType`] for details.
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
primary_type_from! { WType, Wildcard }
primary_type_from! { VcType, Vc }
primary_type_from! { LblType, Lbl }

impl std::fmt::Display for PrimaryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrimaryType::Int(itype) => itype.fmt(f),
            PrimaryType::Float(ftype) => ftype.fmt(f),
            PrimaryType::Ext(ext_type) => ext_type.fmt(f),
            PrimaryType::Ptr(ptr_type) => ptr_type.fmt(f),
            PrimaryType::Wildcard(w_type) => w_type.fmt(f),
            PrimaryType::Vc(vc_type) => vc_type.fmt(f),
            PrimaryType::Lbl(lbl_type) => lbl_type.fmt(f),
        }
    }
}

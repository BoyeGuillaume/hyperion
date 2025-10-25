#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use strum::{EnumIs, EnumTryAs};

/// Represents an integer type with a specified number of bits.
///
/// Signeness is not represented as part of the type; it is determined by the context in which
/// the integer is used. It comes down to the instructions that operate on the integer values.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
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

impl From<u32> for IType {
    fn from(num_bits: u32) -> Self {
        Self::new(num_bits).expect("Invalid number of bits for IntType")
    }
}

impl From<IType> for u32 {
    fn from(int_type: IType) -> Self {
        int_type.num_bits
    }
}

impl std::fmt::Display for IType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "i{}", self.num_bits)
    }
}

/// Represents a floating-point type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIs)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum FpType {
    /// 16-bit floating point value (IEEE-754 binary16)
    Fp16,

    /// 16-bit "brain" floating point value (7-bit significand). Provides
    /// the same number of exponent bits as `FpType::Fp32`, so that it matches the
    /// dynamic range but with greatly reduced precision. Used in
    /// Intel's AVX-512 BF16 extensions and ARM's ARMv8.6-A extensions, among others.
    Bf16,

    /// 32-bit floating point value (IEEE-754 binary32)
    Fp32,

    /// 64-bit floating point value (IEEE-754 binary64)
    Fp64,

    /// 128-bit floating point value (IEEE-754 binary128)
    Fp128,

    /// 80-bit floating point value (X87)
    X86Fp80,

    /// 128-bit floating point value (two 64-bits)
    PPCFp128,
}

impl std::fmt::Display for FpType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            FpType::Fp16 => "half",
            FpType::Bf16 => "bf16",
            FpType::Fp32 => "float",
            FpType::Fp64 => "double",
            FpType::Fp128 => "fp128",
            FpType::X86Fp80 => "x86_fp80",
            FpType::PPCFp128 => "ppc_fp128",
        };
        write!(f, "{}", s)
    }
}

/// The pointer type is used to specify a memory locations. Pointers
/// are commonly used to reference objects in memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PtrType;

/// Primary base types used for Vector types and arrays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum PrimaryBaseType {
    Int(IType),
    Fp(FpType),
    Ptr(PtrType),
}

impl From<IType> for PrimaryBaseType {
    fn from(int_type: IType) -> Self {
        PrimaryBaseType::Int(int_type)
    }
}

impl From<FpType> for PrimaryBaseType {
    fn from(fp_type: FpType) -> Self {
        PrimaryBaseType::Fp(fp_type)
    }
}

impl From<PtrType> for PrimaryBaseType {
    fn from(ptr_type: PtrType) -> Self {
        PrimaryBaseType::Ptr(ptr_type)
    }
}

/// Size of a vector type, either fixed or scalable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum VTypeSize {
    /// Fixed size vector with a number of elements known at compile time.
    Fixed(usize),
    /// Scalable size vector where the number of elements is a multiple of this value.
    VScale(usize),
}

/// Vector Type are a simple derived type that represents a vector of elements.
/// They are typically used in SIMD operations to represent multiple data elements
/// that can be processed in parallel.
///
/// A vector type requires a size, an underlying primitive type, and a scalable property
/// to represent vectors where the exact hardware vector length is unknown at compile time.
///
/// Those types are still considered primary as they can be used directly by instructions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VcType {
    pub size: VTypeSize,
    pub element_type: PrimaryBaseType,
}

impl VcType {
    /// Creates a new fixed-size vector type.
    pub fn fixed(size: usize, element_type: PrimaryBaseType) -> Self {
        Self {
            size: VTypeSize::Fixed(size),
            element_type,
        }
    }

    /// Creates a new scalable vector type.
    pub fn scalable(factor: usize, element_type: PrimaryBaseType) -> Self {
        Self {
            size: VTypeSize::VScale(factor),
            element_type,
        }
    }
}

/// Represents a label used for identifying basic blocks or other entities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct LblType;

/// Represents any primary type in hyperion
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIs, EnumTryAs)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum PrimaryType {
    /// Integer type with specified number of bits.
    Int(IType),

    /// Floating-point type.
    Fp(FpType),

    /// Pointer type.
    Ptr(PtrType),

    /// Vector type.
    Vec(VcType),

    /// Label type.
    Lbl(LblType),
}

impl From<IType> for PrimaryType {
    fn from(int_type: IType) -> Self {
        PrimaryType::Int(int_type)
    }
}

impl From<FpType> for PrimaryType {
    fn from(fp_type: FpType) -> Self {
        PrimaryType::Fp(fp_type)
    }
}

impl From<PtrType> for PrimaryType {
    fn from(ptr_type: PtrType) -> Self {
        PrimaryType::Ptr(ptr_type)
    }
}

impl From<VcType> for PrimaryType {
    fn from(v_type: VcType) -> Self {
        PrimaryType::Vec(v_type)
    }
}

impl From<LblType> for PrimaryType {
    fn from(label: LblType) -> Self {
        PrimaryType::Lbl(label)
    }
}

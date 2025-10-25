use crate::types::primary::{FpType, IType};

/// Represents an integer constant with a specific type and value.
///
/// Notice that in hyperion, integer constants cannot exceed 64 bits in value, for
/// simplicity. We can always build larger integers using other constructs if needed.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct IConst {
    pub int_type: IType,
    pub value: u64,
}

impl IConst {
    #[inline]
    pub fn verify_debug_assert(&self) {
        debug_assert!(
            self.verify(),
            "IntegerConstant value {} exceeds maximum for type {}",
            self.value,
            self.int_type
        );
    }

    #[inline]
    pub fn verify_panic(&self) {
        if !self.verify() {
            panic!(
                "IntegerConstant value {} exceeds maximum for type {}",
                self.value, self.int_type
            );
        }
    }

    #[inline]
    pub const fn verify(&self) -> bool {
        match self.int_type.max_value() {
            Some(max) => self.value <= max,
            None => true, // No limit for integers larger than 64 bits
        }
    }

    #[inline]
    pub const fn new(int_type: IType, value: u64) -> Option<Self> {
        let constant = Self { int_type, value };
        if constant.verify() {
            Some(constant)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FpConstDef {
    Regular {
        mantissa: u64,
        exponent: i32,
        sign: bool,
    },
    Infinity {
        sign: bool,
    },
    NaN,
}

/// Represents a floating-point constant.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct FpConst {
    pub fp_type: FpType,
    pub def: FpConstDef,
}

impl FpConst {
    /// Creates a new `FloatConstant`.
    #[inline]
    pub const fn new(mantissa: u64, exponent: i32, sign: bool, fp_type: FpType) -> Self {
        Self {
            fp_type,
            def: FpConstDef::Regular {
                mantissa,
                exponent,
                sign,
            },
        }
    }
}

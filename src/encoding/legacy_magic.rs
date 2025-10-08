//! Magic constants for raw encoding of DType, Expr, and Prop in RPN form.
//!
//! Conventions:
//! - Trees are encoded in Reverse Polish Notation (postfix): children first, then operator byte.
//! - For binary operators, append the right operand length (u64 varint) right after the right child.
//! - For ternary operators (e.g., If), append the lengths of the last two children: len(child3), len(child2).
//! - Inline variables use a single VAR opcode across Expr and DType; context determines meaning.

//
pub const T_BOOL: u8 = 0x01;
pub const T_OMEGA: u8 = 0x02;
pub const T_FUNC: u8 = 0x05;
pub const T_POWER: u8 = 0x07;

// Expr opcodes
pub const E_NEVER: u8 = 0x10;
pub const E_APP: u8 = 0x11;
pub const E_IF: u8 = 0x12;
pub const E_TUPLE: u8 = 0x13;

// Prop opcodes
pub const P_TRUE: u8 = 0x20;
pub const P_FALSE: u8 = 0x21;
pub const P_NOT: u8 = 0x22;
pub const P_AND: u8 = 0x23;
pub const P_OR: u8 = 0x24;
pub const P_IMPLIES: u8 = 0x25;
pub const P_IFF: u8 = 0x26;
pub const P_FORALL: u8 = 0x27;
pub const P_EXISTS: u8 = 0x28;
pub const P_EQUAL: u8 = 0x29;

// Shared variable opcode (context decides whether it's a DType or Expr var)
pub const MISC_VAR: u8 = 0xf0;
pub const MISC_NOP: u8 = 0xff;

//! Magic constants for raw encoding of DType, Expr, and Prop in RPN form.
//!
//! Conventions:
//! - Trees are encoded in Reverse Polish Notation (postfix): children first, then operator byte.
//! - For binary operators, append the right operand length (u64 varint) right after the right child.
//! - For ternary operators (e.g., If), append the lengths of the last two children: len(child3), len(child2).
//! - Inline variables use a single VAR opcode across Expr and DType; context determines meaning.

// DType opcodes
pub const T_BOOL: u8 = 0x01;
pub const T_OMEGA: u8 = 0x02;
pub const T_NEVER: u8 = 0x03;
pub const T_ARROW: u8 = 0x05; // encode: A B len(B) OP
pub const T_TUPLE: u8 = 0x06; // encode: A B len(B) OP
pub const T_POWER: u8 = 0x07; // encode: A OP

// Expr opcodes
pub const E_UNREACHABLE: u8 = 0x10;
pub const E_APP: u8 = 0x11; // encode: arg payload(func_id) OP
pub const E_IF: u8 = 0x12; // encode: cond then else len(else) len(then) OP
pub const E_TUPLE: u8 = 0x13; // encode: A B len(B) OP

// Prop opcodes
pub const P_TRUE: u8 = 0x20;
pub const P_FALSE: u8 = 0x21;
pub const P_NOT: u8 = 0x22; // encode: P OP
pub const P_AND: u8 = 0x23; // encode: P1 P2 len(P2) OP
pub const P_OR: u8 = 0x24; // encode: P1 P2 len(P2) OP
pub const P_IMPLIES: u8 = 0x25; // encode: P1 P2 len(P2) OP
pub const P_IFF: u8 = 0x26; // encode: P1 P2 len(P2) OP
pub const P_FORALL: u8 = 0x27; // encode: dtype inner len(inner) payload(var_id) OP
pub const P_EXISTS: u8 = 0x28; // encode: dtype inner len(inner) payload(var_id) OP
pub const P_EQUAL: u8 = 0x29; // encode: T1 T2 len(T2) OP

// Shared variable opcode (context decides whether it's a DType or Expr var)
pub const MISC_VAR: u8 = 0xf0; // payload: InlineVariable id (u64 varint)
pub const MISC_NOP: u8 = 0xff; // no payload, no-op (for padding)

/// Expression type enum used to disambiguate variable opcodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpressionType {
    Expr,
    Prop,
    DType,
}

/// Determine expression type from variable opcode byte.
pub fn expr_type_from_var_opcode(op: u8) -> Option<ExpressionType> {
    match op & 0xF0 {
        0x00 => Some(ExpressionType::DType),
        0x10 => Some(ExpressionType::Expr),
        0x20 => Some(ExpressionType::Prop),
        _ => None,
    }
}

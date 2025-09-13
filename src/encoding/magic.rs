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
pub const E_UNREACHABLE: u8 = 0x20;
pub const E_APP: u8 = 0x21; // encode: arg payload(func_id) OP
pub const E_IF: u8 = 0x22; // encode: cond then else len(else) len(then) OP
pub const E_TUPLE: u8 = 0x23; // encode: A B len(B) OP

// Prop opcodes
pub const P_TRUE: u8 = 0x40;
pub const P_FALSE: u8 = 0x41;
pub const P_NOT: u8 = 0x42; // encode: P OP
pub const P_AND: u8 = 0x43; // encode: P1 P2 len(P2) OP
pub const P_OR: u8 = 0x44; // encode: P1 P2 len(P2) OP
pub const P_IMPLIES: u8 = 0x45; // encode: P1 P2 len(P2) OP
pub const P_IFF: u8 = 0x46; // encode: P1 P2 len(P2) OP
pub const P_FORALL: u8 = 0x47; // encode: dtype inner len(inner) payload(var_id) OP
pub const P_EXISTS: u8 = 0x48; // encode: dtype inner len(inner) payload(var_id) OP
pub const P_EQUAL: u8 = 0x49; // encode: T1 T2 len(T2) OP

// Shared variable opcode (context decides whether it's a DType or Expr var)
pub const VAR_INLINE: u8 = 0x10; // payload: InlineVariable id (u64 varint)

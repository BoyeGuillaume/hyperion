use crate::consts::{fp::FConst, int::IConst};

pub mod fp;
pub mod int;

pub enum AnyConst {
    Int(IConst),
    Float(FConst),
}

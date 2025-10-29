use crate::{name::Name, types::aggregate::TypeRef};

#[derive(PartialEq, Clone, Debug, Hash)]
pub enum Operand {
    Local { name: Name, ty: TypeRef },
}

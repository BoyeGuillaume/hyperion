use smallvec::SmallVec;

use crate::encoding::DynBuf;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd)]
pub enum WalkerType {
    DepthFirst,
    BreadthFirst,
}

struct WalkerFrame {
    pos: u64,
    delta: i64,
}

pub struct WalkerNode<'a> {
    walker: &'a mut Walker<'a>,
}

pub struct Walker<'a> {
    buffer: &'a mut DynBuf,
    stackframe: SmallVec<[WalkerFrame; 16]>,
    walker_type: WalkerType,
}

// impl<'a> Walker<'a> {
//     pub fn new(buffer: &'a mut DynBuf, walk) -> Self {
//         Self {
//             buffer,
//             stackframe: SmallVec::new(),
//         }
//     }
// }

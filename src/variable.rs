use crate::encoding::RawEncodable;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InlineVariable(u64);

impl InlineVariable {
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    pub fn id(&self) -> u64 {
        self.0
    }

    pub fn symbol(&self, variant: bool) -> Option<char> {
        let base = if variant { b'a' } else { b'A' };
        if self.0 < 26 {
            Some((base + (self.0 as u8)) as char)
        } else {
            None
        }
    }
}

impl std::fmt::Display for InlineVariable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(c) = self.symbol(f.alternate()) {
            write!(f, "{}", c)
        } else {
            write!(f, "v{}", self.0)
        }
    }
}

impl RawEncodable for InlineVariable {
    fn encode_raw(&self, buf: &mut crate::encoding::DynBuf) {
        crate::encoding::integer::encode_u64(self.id(), buf);
        buf.push(crate::encoding::magic::VAR_INLINE);
    }
}

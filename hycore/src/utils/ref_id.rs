use std::{ops::Deref, sync::Arc};

#[derive(Debug, Clone)]
pub struct RefId<U, T: AsRef<U>> {
    inner: T,
    _phantom: std::marker::PhantomData<U>,
}

impl<U, T: AsRef<U>> RefId<U, T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn borrow_arc(&self) -> &T {
        &self.inner
    }

    pub fn take(self) -> T {
        self.inner
    }
}

impl<U, T: AsRef<U>> AsRef<U> for RefId<U, T> {
    fn as_ref(&self) -> &U {
        self.inner.as_ref()
    }
}

impl<U, T: AsRef<U>> Deref for RefId<U, T> {
    type Target = U;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref()
    }
}

impl<U, T: AsRef<U>> PartialEq for RefId<U, T> {
    fn eq(&self, other: &Self) -> bool {
        let a = self.inner.as_ref() as *const U;
        let b = other.inner.as_ref() as *const U;
        std::ptr::eq(a, b)
    }
}

impl<U, T: AsRef<U>> PartialOrd for RefId<U, T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let a = self.inner.as_ref() as *const U;
        let b = other.inner.as_ref() as *const U;
        Some(a.cmp(&b))
    }
}

impl<U, T: AsRef<U>> Eq for RefId<U, T> {}

impl<U, T: AsRef<U>> Ord for RefId<U, T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl<U, T: AsRef<U>> std::hash::Hash for RefId<U, T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let ptr = self.inner.as_ref() as *const U;
        ptr.hash(state);
    }
}

pub type ArcRefId<U> = RefId<U, Arc<U>>;
pub type RcRefId<U> = RefId<U, std::rc::Rc<U>>;
pub type PtrId<'a, U> = RefId<U, &'a U>;
pub type PtrArcId<'a, U> = RefId<U, &'a Arc<U>>;
// pub type PtrRcId<'a, U> = RefId<U, &'a std::rc::Rc<U>>;

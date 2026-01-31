use std::{ops::Deref, sync::Arc};

/// A reference identifier that uniquely identifies a reference to a value of type `U`.
///
/// This struct wraps a reference-like type `T` (e.g., `Arc<U>`, `&U`) and provides
/// pointer-based equality and ordering based on the address of the underlying `U`.
#[derive(Debug, Clone)]
pub struct RefId<U, T: AsRef<U>> {
    inner: T,
    _phantom: std::marker::PhantomData<U>,
}

impl<U, T: AsRef<U>> RefId<U, T> {
    /// Wraps a reference-like type so pointer identity can be compared or hashed.
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Borrows the underlying reference wrapper (e.g. `Arc`).
    pub fn borrow_arc(&self) -> &T {
        &self.inner
    }

    /// Consumes the wrapper and returns the owned reference-like value.
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

#[allow(clippy::non_canonical_partial_ord_impl)]
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

/// Shortcut for wrapping an `Arc<U>`.
pub type ArcRefId<U> = RefId<U, Arc<U>>;
/// Shortcut for wrapping an `Rc<U>`.
pub type RcRefId<U> = RefId<U, std::rc::Rc<U>>;
/// Shortcut for wrapping a raw reference so it can participate in maps/sets.
pub type PtrId<'a, U> = RefId<U, &'a U>;
/// Shortcut for wrapping a borrowed `Arc<U>` reference.
pub type PtrArcId<'a, U> = RefId<U, &'a Arc<U>>;
// pub type PtrRcId<'a, U> = RefId<U, &'a std::rc::Rc<U>>;

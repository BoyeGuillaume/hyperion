//! Small fixed-capacity vector without heap allocations.
//!
//! Role
//! - Used internally to hold small lists (like node references) without `Vec` allocations.
//! - Capacity is a const generic `N`; length tracked in a `u16` for compactness.
//!
//! Performance
//! - Push/Pop are O(1); insert/erase are O(n) shifting elements up to `N`.
use std::{mem::MaybeUninit, slice::SliceIndex};

/// Fixed-capacity vector backed by a stack-allocated array of `MaybeUninit<T>`.
pub struct StaticVec<T, const N: usize> {
    data: [MaybeUninit<T>; N],
    len: u16,
}

impl<T, const N: usize> StaticVec<T, N> {
    /// Create an empty vector.
    pub fn new() -> Self {
        assert!(N <= u16::MAX as usize, "StaticVec size cannot exceed 65535");
        Self {
            data: unsafe { MaybeUninit::uninit().assume_init() },
            len: 0,
        }
    }

    /// Append `value` to the end. Panics if capacity is exceeded.
    pub fn push(&mut self, value: T) {
        assert!(self.len < N as u16, "StaticVec overflow");
        self.data[self.len as usize] = MaybeUninit::new(value);
        self.len += 1;
    }

    /// Remove and return the last element, or `None` if empty.
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            Some(unsafe { self.data[self.len as usize].as_ptr().read() })
        }
    }

    /// Current number of initialized items.
    pub fn len(&self) -> usize {
        self.len as usize
    }

    /// Whether the vector is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Immutable slice of initialized elements.
    pub fn as_slice(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.data.as_ptr() as *const T, self.len as usize) }
    }

    /// Mutable slice of initialized elements.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe {
            std::slice::from_raw_parts_mut(self.data.as_mut_ptr() as *mut T, self.len as usize)
        }
    }

    /// Iterator over immutable references.
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.as_slice().iter()
    }

    /// Iterator over mutable references.
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        self.as_mut_slice().iter_mut()
    }

    /// Drop all initialized elements, keeping capacity.
    pub fn clear(&mut self) {
        if std::mem::needs_drop::<T>() {
            for i in 0..self.len as usize {
                unsafe {
                    std::ptr::drop_in_place(self.data[i].as_mut_ptr());
                }
            }
        }
        self.len = 0;
    }

    /// Insert `value` at `index`, shifting elements to the right. Panics on overflow or OOB.
    pub fn insert(&mut self, index: usize, value: T) {
        assert!(self.len < N as u16, "StaticVec overflow");
        assert!(index <= self.len as usize, "Index out of bounds");
        for i in (index..self.len as usize).rev() {
            self.data[i + 1] = MaybeUninit::new(unsafe { self.data[i].as_ptr().read() });
        }
        self.data[index] = MaybeUninit::new(value);
        self.len += 1;
    }

    /// Remove the element at `index`, shifting left. Panics if out of bounds.
    pub fn erase(&mut self, index: usize) {
        assert!(index < self.len as usize, "Index out of bounds");
        if std::mem::needs_drop::<T>() {
            unsafe {
                std::ptr::drop_in_place(self.data[index].as_mut_ptr());
            }
        }
        for i in index..(self.len as usize - 1) {
            self.data[i] = MaybeUninit::new(unsafe { self.data[i + 1].as_ptr().read() });
        }
        self.len -= 1;
    }

    /// Maximum number of elements.
    pub fn capacity(&self) -> usize {
        N
    }
}

impl<T, const N: usize> Drop for StaticVec<T, N> {
    fn drop(&mut self) {
        if std::mem::needs_drop::<T>() {
            for i in 0..self.len as usize {
                unsafe {
                    std::ptr::drop_in_place(self.data[i].as_mut_ptr());
                }
            }
        }
    }
}

impl<T: Clone, const N: usize> Clone for StaticVec<T, N> {
    fn clone(&self) -> Self {
        let mut new_vec = Self::new();
        for item in self.as_slice() {
            new_vec.push(item.clone());
        }
        new_vec
    }
}

impl<T: std::fmt::Debug, const N: usize> std::fmt::Debug for StaticVec<T, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.as_slice()).finish()
    }
}

pub struct StaticVecIntoIterator<T, const N: usize> {
    vec: StaticVec<T, N>,
    index: usize,
}

impl<T, const N: usize> Iterator for StaticVecIntoIterator<T, N> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.vec.len() {
            let item = unsafe { self.vec.data[self.index].as_ptr().read() };
            self.index += 1;
            Some(item)
        } else {
            None
        }
    }
}

impl<T, const N: usize> IntoIterator for StaticVec<T, N> {
    type Item = T;
    type IntoIter = StaticVecIntoIterator<T, N>;

    fn into_iter(self) -> Self::IntoIter {
        StaticVecIntoIterator {
            vec: self,
            index: 0,
        }
    }
}

impl<T, const N: usize> FromIterator<T> for StaticVec<T, N> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut vec = Self::new();
        for item in iter {
            vec.push(item);
        }
        vec
    }
}

impl<T, const N: usize> AsRef<[T]> for StaticVec<T, N> {
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T, const N: usize> std::ops::Deref for StaticVec<T, N> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T, const N: usize> std::ops::DerefMut for StaticVec<T, N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl<T, const N: usize> AsMut<[T]> for StaticVec<T, N> {
    fn as_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

impl<T, const N: usize> Default for StaticVec<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize, I: SliceIndex<[T]>> std::ops::Index<I> for StaticVec<T, N> {
    type Output = I::Output;

    fn index(&self, index: I) -> &I::Output {
        &(**self)[index]
    }
}

impl<T, const N: usize, I: SliceIndex<[T]>> std::ops::IndexMut<I> for StaticVec<T, N> {
    fn index_mut(&mut self, index: I) -> &mut I::Output {
        &mut (**self)[index]
    }
}

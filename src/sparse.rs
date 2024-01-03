use alloc::vec::{IntoIter, Vec};
use core::fmt;
use core::slice::{Iter, IterMut};

// A sparse array, holding up to 17 elements, indexed by nybbles with a special exception for
// elements which are shorter than the "choice point" of the branch node which holds this sparse
// array. This special exception is the "head".
#[derive(Clone, PartialEq, Eq)]
pub struct Sparse<T> {
    index: u32,
    entries: Vec<T>,
}

impl<T: fmt::Debug> fmt::Debug for Sparse<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Sparse {{ index: {:b}, entries: {:?} }}",
            self.index, self.entries
        )
    }
}

impl<T> Sparse<T> {
    #[inline]
    pub fn new() -> Sparse<T> {
        Sparse {
            index: 0,
            entries: Vec::with_capacity(2),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    // Go from a nybble-index to an index in the internal element vector.
    #[inline]
    fn actual(&self, idx: u8) -> usize {
        (self.index & ((1 << idx) - 1)).count_ones() as usize
    }

    // Test whether or not the sparse array contains an element for the given index.
    #[inline]
    pub fn contains(&self, idx: u8) -> bool {
        self.index & (1 << idx) != 0
    }

    // Immutably borrow the corresponding element, if it exists.
    #[inline]
    pub fn get(&self, idx: u8) -> Option<&T> {
        if self.contains(idx) {
            Some(&self.entries[self.actual(idx)])
        } else {
            None
        }
    }

    // Mutably borrow the corresponding element, if it exists.
    #[inline]
    pub fn get_mut(&mut self, idx: u8) -> Option<&mut T> {
        if self.contains(idx) {
            let i = self.actual(idx);
            Some(&mut self.entries[i])
        } else {
            None
        }
    }

    // Immutably borrow the element corresponding to this index if it exists - otherwise, immutably
    // borrow an arbitrary element of the array.
    // TODO: Faster to not branch and just calculate the index and return it?
    #[inline]
    pub fn get_or_any(&self, idx: u8) -> &T {
        if self.contains(idx) {
            &self.entries[self.actual(idx)]
        } else {
            &self.entries[0]
        }
    }

    // Mutably borrow the element corresponding to this index if it exists - otherwise, mutably
    // borrow an arbitrary element of the array.
    // TODO: Faster to not branch and just calculate the index and return it?
    #[inline]
    pub fn get_or_any_mut(&mut self, idx: u8) -> &mut T {
        if self.contains(idx) {
            let i = self.actual(idx);
            &mut self.entries[i]
        } else {
            &mut self.entries[0]
        }
    }

    // Assuming that the array does not already contain an element for this index, insert the
    // given element.
    #[inline]
    pub fn insert(&mut self, idx: u8, elt: T) -> &mut T {
        debug_assert!(!self.contains(idx));
        let i = self.actual(idx);
        self.index |= 1 << idx;
        self.entries.insert(i, elt);
        &mut self.entries[i]
    }

    // Assuming that the array contains this index, remove that index and return the corresponding
    // element.
    #[inline]
    pub fn remove(&mut self, idx: u8) -> T {
        debug_assert!(self.contains(idx));
        let i = self.actual(idx);
        self.index &= !(1 << idx);
        self.entries.remove(i)
    }

    // Clear the array, assuming it has a single element remaining, and return that element.
    #[inline]
    pub fn clear_last(&mut self) -> T {
        debug_assert!(self.len() == 1);
        unsafe { self.entries.pop().unwrap_unchecked() }
    }

    #[inline]
    pub fn iter(&self) -> Iter<T> {
        self.entries.iter()
    }

    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<T> {
        self.entries.iter_mut()
    }
}

impl<T> IntoIterator for Sparse<T> {
    type IntoIter = IntoIter<T>;
    type Item = T;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter()
    }
}

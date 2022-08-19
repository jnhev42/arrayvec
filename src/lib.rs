#![no_std]
#![warn(clippy::pedantic)]
use core::{
    fmt, iter,
    mem::{self, MaybeUninit},
    ops, slice,
};

/// Stack allocated vector type with capacity `C`
pub struct ArrayVec<T, const C: usize> {
    data: [MaybeUninit<T>; C],
    write: usize,
}

impl<T, const C: usize> ArrayVec<T, C> {
    /// Creates a new empty `ArrayVec`
    #[must_use]
    pub fn new() -> Self {
        // SAFETY: this array needs no initialisation because its uninitialised memory
        let data = unsafe { MaybeUninit::<[MaybeUninit<T>; C]>::uninit().assume_init() };
        Self { data, write: 0 }
    }

    /// The maximum number of elements the vector can store
    #[allow(clippy::unused_self)]
    pub const fn capacity(&self) -> usize {
        C
    }

    /// The current number of elements the vector stores
    pub const fn len(&self) -> usize {
        self.write
    }

    pub const fn is_full(&self) -> bool {
        self.write == C
    }

    pub const fn is_empty(&self) -> bool {
        self.write == 0
    }

    // Removes all elements from the vector
    pub fn clear(&mut self) {
        // dropping all
        // SAFETY: all indexes are < write so pointing to initialised memory
        (0..self.write).for_each(|i| unsafe { drop(self.take(i)) });
        self.write = 0;
    }

    /// Copys & returns the value at `index`
    /// # Safety
    /// - The value at `index` must be initialised
    /// - Cannot take from same index twice
    unsafe fn take(&mut self, index: usize) -> T {
        let pos = &mut self.data[index];
        let ret = mem::replace(pos, MaybeUninit::uninit());
        ret.assume_init()
    }

    // TODO: try variants
    /// Removes the value at `index` and returns it, maintaining ordering in the array.
    /// # Panics
    /// If `index >= self.len()` out of bounds
    pub fn remove(&mut self, index: usize) -> T {
        assert!(
            index < self.write,
            "index is {index} but length is {0}",
            self.write
        );
        // SAFETY: index is verified to be less than self.write above
        let ret = unsafe { self.take(index) };
        self.write -= 1;
        // SAFETY: self.write has been decremented so it points to initialised memory
        for i in index..self.write {
            let next = unsafe { self.take(i + 1) };
            self.data[i].write(next);
        }
        ret
    }

    /// Removes the value at `index` and returns it, not maintaining ordering in the array.
    /// # Panics
    /// If `index >= self.len()` out of bounds
    pub fn swap_remove(&mut self, index: usize) -> T {
        assert!(
            index < self.write,
            "index is {index} but length is {0}",
            self.write
        );
        // SAFETY: index is verified to be less than self.write above
        let ret = unsafe { self.take(index) };
        self.write -= 1;
        // SAFETY: self.write has been decremented so it points to initialised memory
        let last = unsafe { self.take(self.write) };
        self.data[index].write(last);
        ret
    }

    /// Insert `item` at `index`
    /// # Panics
    /// If `index >= self.len()` out of bounds
    pub fn insert(&mut self, index: usize, item: T) {
        assert!(
            index < self.write,
            "index is {index} but length is {0}",
            self.write
        );
        // starting at the end and copying to the next index
        // if it weren't reversed this would just make the whole
        // rest of the array be whatever item was inserted at
        for i in (index..self.write).rev() {
            // SAFETY: i < self.write as the reversed range starts
            // at self.write - 1
            let val = unsafe { self.take(i) };
            // LEAK: writes to the next index in memory which
            // is deinitialised either because it is at self.write
            // or has been deinitalised in the previous iteration
            self.data[i + 1].write(val);
        }
        self.write += 1;
        // LEAK: data at index has been shifted forward
        // so data[index] is deinitialised
        self.data[index].write(item);
    }
    // TODO: not the implementation of this worst case O(N^2)
    /// Retains only the elements specified by the predicate.
    /// So where `f(element)` is true an element is kept in the list
    pub fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut(&T) -> bool,
    {
        // not quite a for loop because it doesn't always advance
        let mut i = 0;
        while i < self.write {
            // SAFETY: i < self.write above
            let val = unsafe { self.data[i as usize].assume_init_ref() };
            if f(val) {
                // retain, move to next
                i += 1;
            } else {
                // remove, put next where
                // current is

                self.remove(i as usize);
            }
        }
    }
    /// Appends an item to the end of the vector
    /// # Panics
    /// If the vector is full
    pub fn push(&mut self, item: T) {
        assert!(self.write != C, "stackvec full");
        self.data[self.write].write(item);
        self.write += 1;
    }

    /// Removes and returns
    pub fn pop(&mut self) -> Option<T> {
        (self.write != 0).then(|| {
            self.write -= 1;
            // SAFETY: self.write has been decremented
            // so it now points to initialised memory,
            // this deinitialises this memory
            unsafe { self.take(self.write) }
        })
    }

    pub fn into_array(self) -> Option<[T; C]> {
        (self.write == C).then(|| self.data.map(|i| unsafe { i.assume_init() }))
    }

    pub fn resize<const NEW_C: usize>(mut self) -> Option<ArrayVec<T, NEW_C>> {
        if self.write > NEW_C {
            return None;
        }
        // SAFETY: Doesn't need initialisation as its already initialised
        let mut data = unsafe { MaybeUninit::<[MaybeUninit<T>; NEW_C]>::uninit().assume_init() };
        #[allow(clippy::needless_range_loop)] // silly paperclip, your suggestion doesn't compile
        for i in 0..self.write {
            // SAFETY: indexes lower than self.write are initialised
            data[i].write(unsafe { self.take(i) });
        }
        Some(ArrayVec {
            data,
            write: self.write,
        })
    }

    pub fn as_slice(&self) -> &[T] {
        // TODO: MaybeUninit::slice_assume_init_ref once stabilised
        let slice = &self.data[0..self.write];
        let len = slice.len();
        unsafe { core::slice::from_raw_parts(slice.as_ptr().cast::<T>(), len) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        // TODO: MaybeUninit::slice_assume_init_ref once stabilised
        let slice = &mut self.data[0..self.write];
        let len = slice.len(); // == self.write

        // SAFETY: indexes lower than self.write are initialised and thats the length of the slice here
        unsafe { core::slice::from_raw_parts_mut(slice.as_mut_ptr().cast::<T>(), len) }
    }

    pub fn iter(&self) -> slice::Iter<'_, T> {
        self.as_slice().iter()
    }

    pub fn iter_mut(&mut self) -> slice::IterMut<'_, T> {
        self.as_mut_slice().iter_mut()
    }

    // forbids 0 size ArrayVec because
    // implementation depends on C > 0
    // and it doesn't really make sense anyway
    const _C_NON_ZERO: () = assert!(C != 0, "ArrayVec cannot have a capacity of 0");
}

impl<T, const C: usize> ArrayVec<T, C>
where
    T: Clone,
{
    pub fn extend_from_slice(&mut self, other: &[T]) {
        self.extend(other.iter().cloned());
    }
}

impl<T, const C: usize> IntoIterator for ArrayVec<T, C> {
    type Item = T;
    type IntoIter = core::iter::Map<
        core::iter::Take<core::array::IntoIter<MaybeUninit<T>, C>>,
        fn(MaybeUninit<T>) -> T,
    >;
    fn into_iter(self) -> Self::IntoIter {
        // not actually a safe function
        fn assume_init<T>(val: MaybeUninit<T>) -> T {
            // SAFETY: val is from an index below self.write
            unsafe { val.assume_init() }
        }
        self.data.into_iter().take(self.write - 1).map(assume_init)
    }
}

impl<T, const C: usize> iter::Extend<T> for ArrayVec<T, C> {
    fn extend<U: IntoIterator<Item = T>>(&mut self, iter: U) {
        iter.into_iter().for_each(|i| self.push(i));
    }
}

impl<'a, T, const C: usize> iter::Extend<&'a T> for ArrayVec<T, C>
where
    T: Clone,
{
    fn extend<U: IntoIterator<Item = &'a T>>(&mut self, iter: U) {
        iter.into_iter().for_each(|i| self.push(i.clone()));
    }
}

impl<'a, T, const C: usize> iter::Extend<&'a [T]> for ArrayVec<T, C>
where
    T: Clone,
{
    fn extend<I: IntoIterator<Item = &'a [T]>>(&mut self, iter: I) {
        iter.into_iter().for_each(|i| self.extend_from_slice(i));
    }
}

impl<T, const C: usize> iter::FromIterator<T> for ArrayVec<T, C> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut ret = ArrayVec::new();
        ret.extend(iter);
        ret
    }
}

impl<T, const C: usize> Clone for ArrayVec<T, C>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        // SAFETY: [MaybeUninit<T>] doesn't need to be initialised
        let mut data: [MaybeUninit<T>; C] = unsafe { MaybeUninit::uninit().assume_init() };
        for i in 0..self.write {
            data[i].write(self[i].clone());
        }
        Self {
            data,
            write: self.write,
        }
    }
}

// -------------------- trivial impls -------------------- \\

impl<T, const C: usize> ops::Deref for ArrayVec<T, C> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T, const C: usize> ops::DerefMut for ArrayVec<T, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl<T, const C: usize> fmt::Debug for ArrayVec<T, C>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_slice().fmt(f)
    }
}

impl<T, const C: usize> Default for ArrayVec<T, C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const C: usize> AsMut<[T]> for ArrayVec<T, C> {
    fn as_mut(&mut self) -> &mut [T] {
        self
    }
}

impl<T, const C: usize> AsMut<ArrayVec<T, C>> for ArrayVec<T, C> {
    fn as_mut(&mut self) -> &mut ArrayVec<T, C> {
        self
    }
}

impl<T, const C: usize> AsRef<[T]> for ArrayVec<T, C> {
    fn as_ref(&self) -> &[T] {
        self
    }
}

impl<T, const C: usize> AsRef<ArrayVec<T, C>> for ArrayVec<T, C> {
    fn as_ref(&self) -> &ArrayVec<T, C> {
        self
    }
}

impl<T, const C: usize> core::borrow::Borrow<[T]> for ArrayVec<T, C> {
    fn borrow(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T, const C: usize> core::borrow::BorrowMut<[T]> for ArrayVec<T, C> {
    fn borrow_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

#[cfg(test)]
mod tests;

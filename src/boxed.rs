//! This module contains the [`Box`] smart pointer, capable of managing memory in a [`IndexAllocator`].

use core::alloc::Layout;
use core::mem;
use core::ops::{Deref, DerefMut};
use core::ptr;

use crate::{IndexAllocator, IndexError};

/// A smart pointer holding its value in an [`IndexAllocator`] and managing its memroy.
///
/// The [`Box`] smart pointer can be obtained by using [`IndexAllocator::try_boxed`]
/// or by directly using [`Box::try_new`] and providing the [`IndexAllocator`].
///
/// # Example
///
/// ```
/// use index_alloc::IndexAllocator;
///
/// let allocator: IndexAllocator<64, 8> = IndexAllocator::empty();
///
/// let test_box = allocator.try_boxed([1, 2, 3, 4]).unwrap();
/// assert_eq!(*test_box, [1, 2, 3, 4]);
/// ```
pub struct Box<'a, T, const MEMORY_SIZE: usize, const INDEX_SIZE: usize>
where
    T: ?Sized,
{
    val: &'a mut T,
    allocator: &'a IndexAllocator<MEMORY_SIZE, INDEX_SIZE>,
}

impl<'a, T, const MEMORY_SIZE: usize, const INDEX_SIZE: usize> Box<'a, T, MEMORY_SIZE, INDEX_SIZE>
where
    T: ?Sized,
{
    /// Try to create a new [`Box`] containing a value of type `T` in an [`IndexAllocator`].
    /// See also [`IndexAllocator::try_boxed`] to create a [`Box`] directly by the allocator.
    ///
    /// # Errors
    /// The method return an [`IndexError`] if the allocation failled.
    pub fn try_new<U>(
        val: U,
        allocator: &'a IndexAllocator<MEMORY_SIZE, INDEX_SIZE>,
    ) -> Result<Self, IndexError>
    where
        U: 'a,
        &'a mut T: From<&'a mut U>,
    {
        let layout = Layout::for_value(&val);
        let inner_ptr = allocator.try_alloc(layout)?.cast::<U>();
        let inner_ref = unsafe { inner_ptr.as_mut().ok_or(IndexError::EmptyPtr) }?;
        // Ensure the inner_ref destructor isn't called as it's uninisialized memory.
        mem::forget(mem::replace(inner_ref, val));

        Ok(unsafe { Self::from_raw_ref(inner_ref.into(), allocator) })
    }

    pub unsafe fn from_raw_ref(
        val: &'a mut T,
        allocator: &'a IndexAllocator<MEMORY_SIZE, INDEX_SIZE>,
    ) -> Self {
        Self { val, allocator }
    }

    /// Try to free the memory the [`Box`] is managing, dropping its value.
    ///
    /// # Errors
    ///
    /// The method return a [`IndexError`] if the deallocation failled.
    pub fn try_free(self) -> Result<(), IndexError> {
        self.allocator
            .try_free(ptr::from_mut(self.val).cast::<u8>())
    }

    /// Get a reference to the [`IndexAllocator`] used by the box.
    #[must_use]
    pub fn allocator(&self) -> &'a IndexAllocator<MEMORY_SIZE, INDEX_SIZE> {
        self.allocator
    }
}

impl<'a, T, const MEMORY_SIZE: usize, const INDEX_SIZE: usize> Drop
    for Box<'a, T, MEMORY_SIZE, INDEX_SIZE>
where
    T: ?Sized,
{
    fn drop(&mut self) {
        self.allocator
            .try_free(ptr::from_mut(self.val).cast::<u8>())
            .unwrap();
    }
}

impl<'a, T, const MEMORY_SIZE: usize, const INDEX_SIZE: usize> Deref
    for Box<'a, T, MEMORY_SIZE, INDEX_SIZE>
where
    T: ?Sized,
{
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.val
    }
}

impl<'a, T, const MEMORY_SIZE: usize, const INDEX_SIZE: usize> DerefMut
    for Box<'a, T, MEMORY_SIZE, INDEX_SIZE>
where
    T: ?Sized,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.val
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_box_allocation() {
        let allocator: IndexAllocator<64, 8> = IndexAllocator::empty();

        let test_box = Box::try_new([1u8, 2, 3, 4], &allocator).unwrap();

        assert_eq!(*test_box, [1, 2, 3, 4]);
        assert_eq!(unsafe { (*allocator.memory.get())[0] }, 1);
        assert_eq!(unsafe { (*allocator.memory.get())[1] }, 2);
        assert_eq!(unsafe { (*allocator.memory.get())[2] }, 3);
        assert_eq!(unsafe { (*allocator.memory.get())[3] }, 4);
    }
}

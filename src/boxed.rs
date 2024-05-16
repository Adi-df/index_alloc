//! This module contains the [`Box`] smart pointer, capable of managing memory in a [`IndexAllocator`].

use core::ops::{Deref, DerefMut};

use crate::{IndexAllocator, IndexError};

/// A smart pointer holding its value in an [`IndexAllocator`] and managing its memory.
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
///
/// # Dynamic dispatch
///
/// As for the core library `Box`, this [`Box`] type can be used for owned unsized types.
/// This allows the use of types such as `Box<dyn Trait>`.
///
/// Note that to do that, it relies on some lifetime and type magic and on the fact conversion from `&mut T` to
/// `&mut dyn Trait` where `T` implements `Trait`, this conversion is done with the [`From`] trait
/// but, it may be necessary to implement the trait manually.
/// For more information, see the [`Dynamic dispatch example`].
///
/// [`Dynamic dispatch example`]: https://github.com/Adi-df/index_alloc/blob/master/examples/dynamic_dispatch_example.rs
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
    /// The method return an [`IndexError`] if the allocation failed.
    pub fn try_new<U>(
        val: U,
        allocator: &'a IndexAllocator<MEMORY_SIZE, INDEX_SIZE>,
    ) -> Result<Self, IndexError>
    where
        U: 'a,
        &'a mut T: From<&'a mut U>,
    {
        let inner_ref = unsafe { allocator.try_alloc_value(val)? };

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
    /// The method return a [`IndexError`] if the deallocation failed.
    pub fn try_free(self) -> Result<(), IndexError> {
        unsafe { self.allocator.try_free_value(self.val) }
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
        unsafe {
            self.allocator.try_free_value(self.val).unwrap();
        }
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

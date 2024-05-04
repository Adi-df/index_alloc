use core::alloc::Layout;
use core::ops::{Deref, DerefMut};
use core::ptr;

use crate::{IndexAllocator, IndexError};

pub struct Box<'a, T, const MEMORY_SIZE: usize, const INDEX_SIZE: usize> {
    val: &'a mut T,
    allocator: &'a IndexAllocator<MEMORY_SIZE, INDEX_SIZE>,
}

impl<'a, T, const MEMORY_SIZE: usize, const INDEX_SIZE: usize> Box<'a, T, MEMORY_SIZE, INDEX_SIZE> {
    pub fn try_new(
        val: T,
        allocator: &'a IndexAllocator<MEMORY_SIZE, INDEX_SIZE>,
    ) -> Result<Self, IndexError> {
        let layout = Layout::for_value(&val);
        let inner_ptr = allocator.try_alloc(layout)?.cast::<T>();
        let inner_ref = unsafe { inner_ptr.as_mut().ok_or(IndexError::EmptyPtr) }?;
        *inner_ref = val;

        Ok(Self {
            val: inner_ref,
            allocator,
        })
    }

    pub fn try_free(self) -> Result<(), IndexError> {
        self.allocator
            .try_free(ptr::from_mut(self.val).cast::<u8>())
    }
}

impl<'a, T, const MEMORY_SIZE: usize, const INDEX_SIZE: usize> Drop
    for Box<'a, T, MEMORY_SIZE, INDEX_SIZE>
{
    fn drop(&mut self) {
        self.allocator
            .try_free(ptr::from_mut(self.val).cast::<u8>())
            .unwrap();
    }
}

impl<'a, T, const MEMORY_SIZE: usize, const INDEX_SIZE: usize> Deref
    for Box<'a, T, MEMORY_SIZE, INDEX_SIZE>
{
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.val
    }
}

impl<'a, T, const MEMORY_SIZE: usize, const INDEX_SIZE: usize> DerefMut
    for Box<'a, T, MEMORY_SIZE, INDEX_SIZE>
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

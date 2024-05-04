use core::alloc::Layout;
use core::ops::{Deref, DerefMut};

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
        let addr = allocator.try_reserve(layout)?;
        let inner_ref = unsafe {
            allocator
                .memory
                .get()
                .add(addr)
                .cast::<T>()
                .as_mut()
                .unwrap()
        };
        *inner_ref = val;

        Ok(Self {
            val: inner_ref,
            allocator,
        })
    }

    pub fn try_free(self) -> Result<(), IndexError> {
        self.allocator.try_free((self.val as *mut T).cast::<u8>())
    }
}

impl<'a, T, const MEMORY_SIZE: usize, const INDEX_SIZE: usize> Drop
    for Box<'a, T, MEMORY_SIZE, INDEX_SIZE>
{
    fn drop(&mut self) {
        self.allocator
            .try_free((self.val as *mut T).cast::<u8>())
            .unwrap();
    }
}

impl<'a, T, const MEMORY_SIZE: usize, const INDEX_SIZE: usize> Deref
    for Box<'a, T, MEMORY_SIZE, INDEX_SIZE>
{
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.val
    }
}

impl<'a, T, const MEMORY_SIZE: usize, const INDEX_SIZE: usize> DerefMut
    for Box<'a, T, MEMORY_SIZE, INDEX_SIZE>
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.val
    }
}

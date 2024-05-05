use core::alloc::Layout;
use core::cell::Cell;
use core::mem;
use core::ops::Deref;
use core::ptr;

use crate::{IndexAllocator, IndexError};

pub struct Rc<'a, T, const MEMORY_SIZE: usize, const INDEX_SIZE: usize>
where
    T: ?Sized,
{
    val: &'a T,
    strong: &'a Cell<usize>,
    allocator: &'a IndexAllocator<MEMORY_SIZE, INDEX_SIZE>,
}

impl<'a, T, const MEMORY_SIZE: usize, const INDEX_SIZE: usize> Rc<'a, T, MEMORY_SIZE, INDEX_SIZE>
where
    T: ?Sized,
{
    pub fn try_new<U>(
        val: U,
        allocator: &'a IndexAllocator<MEMORY_SIZE, INDEX_SIZE>,
    ) -> Result<Self, IndexError>
    where
        U: 'a,
        &'a T: From<&'a U>,
    {
        let strong_counter = Cell::new(1);
        let val_layout = Layout::for_value(&val);
        let strong_counter_layout = Layout::for_value(&strong_counter);

        let val_ptr = allocator.try_alloc(val_layout)?.cast::<U>();
        let val_ref = unsafe { val_ptr.as_mut().ok_or(IndexError::EmptyPtr) }?;

        let strong_counter_ptr = allocator
            .try_alloc(strong_counter_layout)?
            .cast::<Cell<usize>>();
        let strong_counter_ref =
            unsafe { strong_counter_ptr.as_mut().ok_or(IndexError::EmptyPtr) }?;

        mem::forget(mem::replace(val_ref, val));
        *strong_counter_ref = strong_counter;

        Ok(Self {
            val: <&'a T>::from(&*val_ref),
            strong: &*strong_counter_ref,
            allocator,
        })
    }

    #[must_use]
    pub fn strong_count(&self) -> usize {
        self.strong.get()
    }

    #[must_use]
    pub fn allocator(&self) -> &'a IndexAllocator<MEMORY_SIZE, INDEX_SIZE> {
        self.allocator
    }
}

impl<'a, T, const MEMORY_SIZE: usize, const INDEX_SIZE: usize> Clone
    for Rc<'a, T, MEMORY_SIZE, INDEX_SIZE>
where
    T: ?Sized,
{
    #[must_use]
    fn clone(&self) -> Self {
        self.strong.set(self.strong.get() + 1);
        Self { ..*self }
    }
}

impl<'a, T, const MEMORY_SIZE: usize, const INDEX_SIZE: usize> Deref
    for Rc<'a, T, MEMORY_SIZE, INDEX_SIZE>
where
    T: ?Sized,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.val
    }
}

impl<'a, T, const MEMORY_SIZE: usize, const INDEX_SIZE: usize> Drop
    for Rc<'a, T, MEMORY_SIZE, INDEX_SIZE>
where
    T: ?Sized,
{
    fn drop(&mut self) {
        self.strong.set(self.strong.get() - 1);
        if self.strong.get() == 0 {
            self.allocator
                .try_free(ptr::from_ref(self.val).cast_mut().cast::<u8>())
                .unwrap();
            self.allocator
                .try_free(ptr::from_ref(self.strong).cast_mut().cast::<u8>())
                .unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::index::MemoryRegion;

    use super::*;

    #[test]
    fn test_rc_allocation() {
        let allocator: IndexAllocator<64, 8> = IndexAllocator::empty();

        let test_rc = Rc::try_new([1u8, 2, 3, 4], &allocator).unwrap();

        assert_eq!(*test_rc, [1, 2, 3, 4]);
        assert_eq!(
            allocator.index.borrow().get_region(0),
            Ok(&MemoryRegion::new(0, 4, true))
        );

        drop(test_rc);

        assert_eq!(
            allocator.index.borrow().get_region(0),
            Ok(&MemoryRegion::new(0, 64, false))
        );
    }

    #[test]
    fn test_rc_counting() {
        let allocator: IndexAllocator<64, 8> = IndexAllocator::empty();

        let test_rc = Rc::try_new("Hello world", &allocator).unwrap();

        assert_eq!(test_rc.strong_count(), 1);

        {
            let rc_clone = Rc::clone(&test_rc);

            assert_eq!(rc_clone.strong_count(), 2);
        }

        assert_eq!(test_rc.strong_count(), 1);
        assert_eq!(*test_rc, "Hello world");
    }
}

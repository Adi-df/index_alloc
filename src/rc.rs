use core::cell::Cell;
use core::ops::Deref;

use crate::{IndexAllocator, IndexError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RcError {
    TryToFreeEmptyRcBox,
    IndexError(IndexError),
}

struct RcBox<'a, T, const MEMORY_SIZE: usize, const INDEX_SIZE: usize>
where
    T: ?Sized,
{
    pub val: Cell<Option<&'a T>>,
    pub strong: Cell<usize>,
    allocator: &'a IndexAllocator<MEMORY_SIZE, INDEX_SIZE>,
}

impl<'a, T, const MEMORY_SIZE: usize, const INDEX_SIZE: usize> RcBox<'a, T, MEMORY_SIZE, INDEX_SIZE>
where
    T: ?Sized,
{
    fn try_new<U>(
        val: U,
        allocator: &'a IndexAllocator<MEMORY_SIZE, INDEX_SIZE>,
    ) -> Result<Self, IndexError>
    where
        U: 'a,
        &'a T: From<&'a U>,
    {
        let val_ref = unsafe { allocator.try_alloc_value(val)? };

        Ok(Self {
            val: Cell::new(Some(<&'a T>::from(&*val_ref))),
            strong: Cell::new(0),
            allocator,
        })
    }

    fn allocator(&self) -> &'a IndexAllocator<MEMORY_SIZE, INDEX_SIZE> {
        self.allocator
    }

    fn increment_strong(&self) {
        self.strong.set(self.strong.get() + 1);
    }

    fn decrement_strong(&self) {
        self.strong.set(self.strong.get() - 1);
    }

    fn try_free_inner(&self) -> Result<(), RcError> {
        match self.val.get() {
            Some(v) => {
                unsafe {
                    self.allocator
                        .try_free_value(v)
                        .map_err(RcError::IndexError)?;
                }
                self.val.set(None);
                Ok(())
            }
            None => Err(RcError::TryToFreeEmptyRcBox),
        }
    }
}

impl<'a, T, const MEMORY_SIZE: usize, const INDEX_SIZE: usize> Drop
    for RcBox<'a, T, MEMORY_SIZE, INDEX_SIZE>
where
    T: ?Sized,
{
    fn drop(&mut self) {
        if let Some(v) = self.val.get() {
            unsafe {
                self.allocator
                    .try_free_value(v)
                    .map_err(RcError::IndexError)
                    .unwrap();
            }
        }
    }
}

pub struct Rc<'a, T, const MEMORY_SIZE: usize, const INDEX_SIZE: usize>
where
    T: ?Sized,
{
    rc_box: &'a RcBox<'a, T, MEMORY_SIZE, INDEX_SIZE>,
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
        let rc_box = RcBox::try_new(val, allocator)?;
        rc_box.increment_strong();

        let rc_box_ref = unsafe { allocator.try_alloc_value(rc_box)? };

        Ok(Self { rc_box: rc_box_ref })
    }

    #[must_use]
    pub fn strong_count(&self) -> usize {
        self.rc_box.strong.get()
    }

    #[must_use]
    pub fn allocator(&self) -> &'a IndexAllocator<MEMORY_SIZE, INDEX_SIZE> {
        self.rc_box.allocator()
    }
}

impl<'a, T, const MEMORY_SIZE: usize, const INDEX_SIZE: usize> Clone
    for Rc<'a, T, MEMORY_SIZE, INDEX_SIZE>
where
    T: ?Sized,
{
    #[must_use]
    fn clone(&self) -> Self {
        self.rc_box.increment_strong();
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
        match self.rc_box.val.get() {
            Some(v) => v,
            None => unreachable!(),
        }
    }
}

impl<'a, T, const MEMORY_SIZE: usize, const INDEX_SIZE: usize> Drop
    for Rc<'a, T, MEMORY_SIZE, INDEX_SIZE>
where
    T: ?Sized,
{
    fn drop(&mut self) {
        self.rc_box.decrement_strong();
        if self.rc_box.strong.get() == 0 {
            self.rc_box.try_free_inner().unwrap();

            unsafe {
                self.allocator().try_free_value(self.rc_box).unwrap();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rc_allocation() {
        let allocator: IndexAllocator<64, 8> = IndexAllocator::empty();

        let test_rc = Rc::try_new([1u8, 2, 3, 4], &allocator).unwrap();

        assert_eq!(*test_rc, [1, 2, 3, 4]);
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

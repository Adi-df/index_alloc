//! This module contains the [`Rc`] smart point capable of shared ownership of memory in a [`IndexAllocator`]

use core::cell::Cell;
use core::ops::Deref;

use crate::{IndexAllocator, IndexError};

/// A smart pointer holding it's value in a [`IndexAllocator`] and managing its memory.
/// It also keep track of the number of strong and weak references to the inner value.
struct RcBox<'a, T, const MEMORY_SIZE: usize, const INDEX_SIZE: usize>
where
    T: ?Sized,
{
    pub val: Cell<Option<&'a T>>,
    pub strong: Cell<usize>,
    pub weak: Cell<usize>,
    allocator: &'a IndexAllocator<MEMORY_SIZE, INDEX_SIZE>,
}

impl<'a, T, const MEMORY_SIZE: usize, const INDEX_SIZE: usize> RcBox<'a, T, MEMORY_SIZE, INDEX_SIZE>
where
    T: ?Sized,
{
    /// Allocate the inner type and set the strong and weak count to 0.
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
            weak: Cell::new(0),
            allocator,
        })
    }

    /// Try to free the inner value and set it to None.
    /// Panic if the inner value is already freed. (Which shouldn't happen).
    fn try_free_inner(&self) -> Result<(), IndexError> {
        match self.val.get() {
            Some(v) => {
                unsafe {
                    self.allocator.try_free_value(v)?;
                }
                self.val.set(None);
                Ok(())
            }
            None => unreachable!(),
        }
    }

    fn increment_strong(&self) {
        self.strong.set(self.strong.get() + 1);
    }

    fn decrement_strong(&self) {
        self.strong.set(self.strong.get() - 1);
    }

    fn increment_weak(&self) {
        self.weak.set(self.weak.get() + 1);
    }

    fn decrement_weak(&self) {
        self.weak.set(self.weak.get() - 1);
    }

    /// Return the inner allocator used.
    fn allocator(&self) -> &'a IndexAllocator<MEMORY_SIZE, INDEX_SIZE> {
        self.allocator
    }
}

impl<'a, T, const MEMORY_SIZE: usize, const INDEX_SIZE: usize> Drop
    for RcBox<'a, T, MEMORY_SIZE, INDEX_SIZE>
where
    T: ?Sized,
{
    /// Drop the inner value if not already done.
    fn drop(&mut self) {
        if let Some(v) = self.val.get() {
            unsafe {
                self.allocator.try_free_value(v).unwrap();
                self.val.set(None);
            }
        }
    }
}

/// A smart pointer holding its value in an [`IndexAllocator`] and allowing shared ownership between multiple [`Rc`].
///
/// The [`Rc`] smart pointer can be obtained by using [`Rc::try_new`].
///
/// # Example
///
/// ```
/// use index_alloc::IndexAllocator;
/// use index_alloc::rc::Rc;
///
/// let allocator: IndexAllocator<64, 8> = IndexAllocator::empty();
///
/// let test_rc = Rc::try_new([1, 2, 3, 4], &allocator).unwrap();
/// assert_eq!(*test_rc, [1, 2, 3, 4]);
/// ```
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
    /// Try to create a new [`Rc`] owning a value allocated of type `T` on a [`IndexAllocator`].
    /// The inner memory is reference counted and only freed when every strong reference ([`Rc`]) are dropped.
    ///
    /// # Errors
    /// The method return an [`IndexError`] if the allocation failed.
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

    /// Create a [`Weak`] reference to the value owned by the [`Rc`].
    pub fn downgrade(&self) -> Weak<'a, T, MEMORY_SIZE, INDEX_SIZE> {
        self.rc_box.increment_weak();
        Weak {
            rc_box: self.rc_box,
        }
    }

    /// Return the number of strong reference (see [`Rc`]) to the inner value.
    #[must_use]
    pub fn strong_count(&self) -> usize {
        self.rc_box.strong.get()
    }

    /// Return the number of weak reference (see [`Weak`]) to the inner value.
    #[must_use]
    pub fn weak_count(&self) -> usize {
        self.rc_box.weak.get()
    }

    /// Get a reference to the [`IndexAllocator`] used by the [`Rc`].
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
    /// Create a new [`Rc`] referencing to the same value.
    ///
    /// # Example
    ///
    /// ```
    /// use index_alloc::IndexAllocator;
    /// use index_alloc::rc::Rc;
    ///
    /// let allocator: IndexAllocator<64, 8> = IndexAllocator::empty();
    ///
    /// let test_rc = Rc::try_new("Hello World", &allocator).unwrap();
    ///
    /// {
    ///     let test_ref = Rc::clone(&test_rc);
    ///     assert_eq!(*test_ref, "Hello World");
    /// }
    /// ```
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
        // If the strong count get to 0, drop the inner value.
        if self.rc_box.strong.get() == 0 {
            self.rc_box.try_free_inner().unwrap();

            // If morover the weak count gets to 0, drop the inner box.
            if self.rc_box.weak.get() == 0 {
                unsafe {
                    self.allocator().try_free_value(self.rc_box).unwrap();
                }
            }
        }
    }
}

/// A smart pointer to a value in an [`Rc`] which doesn't hold the inner data.
/// As the inner data can be dropped when no more [`Rc`] are holding it,
/// a [`Weak`] reference can't directly access it's inner data and must be upgraded to an [`Rc`] with the [`Weak::upgrade`] method.
///
/// The [`Weak`] smart pointer can be obtained by using the [`Rc::downgrade`] method.
///
/// # Example
///
/// ```
/// use index_alloc::IndexAllocator;
/// use index_alloc::rc::Rc;
///
/// let allocator: IndexAllocator<64, 8> = IndexAllocator::empty();
///
/// let test_rc = Rc::try_new([1, 2, 3, 4], &allocator).unwrap();
/// let test_ref = test_rc.downgrade();
/// assert_eq!(test_ref.strong_count(), 1);
/// ```
pub struct Weak<'a, T, const MEMORY_SIZE: usize, const INDEX_SIZE: usize>
where
    T: ?Sized,
{
    rc_box: &'a RcBox<'a, T, MEMORY_SIZE, INDEX_SIZE>,
}

impl<'a, T, const MEMORY_SIZE: usize, const INDEX_SIZE: usize> Weak<'a, T, MEMORY_SIZE, INDEX_SIZE>
where
    T: ?Sized,
{
    /// Try to upgrade the [`Weak`] reference to a strong reference ([`Rc`]) return `None` if the inner_value was already dropped.
    #[must_use]
    pub fn upgrade(&self) -> Option<Rc<'a, T, MEMORY_SIZE, INDEX_SIZE>> {
        if self.strong_count() > 0 {
            self.rc_box.increment_strong();
            Some(Rc {
                rc_box: self.rc_box,
            })
        } else {
            None
        }
    }

    /// Return the number of strong reference (see [`Rc`]) to the inner value.
    #[must_use]
    pub fn strong_count(&self) -> usize {
        self.rc_box.strong.get()
    }

    /// Return the number of weak reference (see [`Weak`]) to the inner value.
    #[must_use]
    pub fn weak_count(&self) -> usize {
        self.rc_box.weak.get()
    }

    /// Get a reference to the [`IndexAllocator`] used by the [`Weak`] reference.
    #[must_use]
    pub fn allocator(&self) -> &'a IndexAllocator<MEMORY_SIZE, INDEX_SIZE> {
        self.rc_box.allocator
    }
}

impl<'a, T, const MEMORY_SIZE: usize, const INDEX_SIZE: usize> Drop
    for Weak<'a, T, MEMORY_SIZE, INDEX_SIZE>
where
    T: ?Sized,
{
    fn drop(&mut self) {
        self.rc_box.decrement_weak();

        // If no more reference (strong or weak), drop the inner box.
        if self.rc_box.strong.get() == 0 && self.rc_box.weak.get() == 0 {
            unsafe {
                self.allocator().try_free_value(self.rc_box).unwrap();
            }
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

    #[test]
    fn test_weak_counting() {
        let allocator: IndexAllocator<64, 8> = IndexAllocator::empty();

        let test_rc = Rc::try_new("Hello World", &allocator).unwrap();
        let test_weak = test_rc.downgrade();

        assert_eq!(test_rc.strong_count(), 1);
        assert_eq!(test_rc.weak_count(), 1);
        assert_eq!(test_weak.strong_count(), 1);
        assert_eq!(test_weak.weak_count(), 1);

        {
            let second_rc = test_weak.upgrade().unwrap();
            assert_eq!(second_rc.weak_count(), 1);
            assert_eq!(second_rc.strong_count(), 2);
        }

        assert_eq!(test_rc.strong_count(), 1);
    }

    #[test]
    fn test_weak_on_dropped_value() {
        let allocator: IndexAllocator<64, 8> = IndexAllocator::empty();

        let test_rc = Rc::try_new("Hello World", &allocator).unwrap();
        let test_weak = test_rc.downgrade();

        drop(test_rc);

        assert_eq!(test_weak.strong_count(), 0);
        assert!(matches!(test_weak.upgrade(), None));

        drop(test_weak);

        assert_eq!(
            allocator.index.borrow().get_region(0),
            Ok(&MemoryRegion::new(0, 64, false))
        );
    }
}

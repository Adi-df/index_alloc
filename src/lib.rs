//! A simple, toy static Allocator wich use a fixed length array to store allocated data.
//!
//! This crate expose a struct [IndexAllocator] wich implement [GlobalAlloc] so it can be uses as the global allocator in no_std environement.
//!
//! Disadvantages :
//! - Extremly unsafe
//! - Very slow
//! - Memory inefficient
//!
//! Even though it seems unusable, it has plenty of advantages :
//! - Just joking don't use that
//!
//! To store allocated memory, [IndexAllocator] uses a MemoryIndex wich stores a list of regions containing the state of the region (size, from which address, used status). For instance :
//!
//! ```rust
//! use index_alloc::IndexAllocator;
//!
//! #[global_allocator]
//! static ALLOCATOR: IndexAllocator<2048, 16> = IndexAllocator::empty();
//!
//! fn main() {
//!     let test_str = String::from("Hello World");
//!     println!("{test_str}");
//! }
//! ```

#![no_std]

use core::alloc::{GlobalAlloc, Layout};
use core::cell::{RefCell, UnsafeCell};

pub mod boxed;
mod index;

use boxed::Box;
use index::MemoryIndex;

/// The Error type wich the Allocator can raise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexError {
    /// The memory region trying to be accessed doesn't exists.
    NoSuchRegion,
    /// The MemoryIndex is full and no more allocation can be performed.
    NoIndexAvailable,
    /// No free region match the allocation needs.
    NoFittingRegion,
    /// The address provided isn't in the memory range.
    OutOfMemory,
    /// The region is too thin for the operation trying to be executed on it.
    RegionTooThin,
    /// The pointer provided is null.
    EmptyPtr,
    /// The MemoryIndex is already borrowed.
    IndexAlreadyBorrowed,
}

/// The [IndexAllocator] struct is the main component of this crate, it create a memory pool of size `MEMORY_SIZE` with an index of size `INDEX_SIZE`.
///
/// There are no restriction on how `MEMORY_SIZE` and `INDEX_SIZE` are set, but `INDEX_SIZE` corresponds to the maximum number of allocated objects that can be held at the same time.
///
/// For instance, setting `INDEX_SIZE` to 4 means no more allocations can be performed after 4 boxes are allocated, except if some of them are freed.
///
/// [IndexAllocator] implement the [GlobalAlloc] trait wich allows it to be used as the app allocator.
///
/// # Example
///
/// ```rust
/// use index_alloc::IndexAllocator;
///
/// #[global_allocator]
/// static ALLOCATOR: IndexAllocator<1024, 16> = IndexAllocator::empty();
///```
pub struct IndexAllocator<const MEMORY_SIZE: usize, const INDEX_SIZE: usize> {
    memory: UnsafeCell<[u8; MEMORY_SIZE]>,
    index: RefCell<MemoryIndex<INDEX_SIZE>>,
}

unsafe impl<const MEMORY_SIZE: usize, const INDEX_SIZE: usize> Sync
    for IndexAllocator<MEMORY_SIZE, INDEX_SIZE>
{
}

impl<const MEMORY_SIZE: usize, const INDEX_SIZE: usize> IndexAllocator<MEMORY_SIZE, INDEX_SIZE> {
    #[must_use]
    const fn new(memory: [u8; MEMORY_SIZE], index: MemoryIndex<INDEX_SIZE>) -> Self {
        Self {
            memory: UnsafeCell::new(memory),
            index: RefCell::new(index),
        }
    }

    /// Creates an empty [IndexAllocator].
    /// Inner memory is just zeroes.
    /// Index is empty.
    ///
    /// This should be the standard way to create an [IndexAllocator].
    ///
    /// Note that the `MEMORY_SIZE` and `INDEX_SIZE` need to be infered at this point.
    #[must_use]
    pub const fn empty() -> Self {
        Self::new([0; MEMORY_SIZE], MemoryIndex::empty(MEMORY_SIZE))
    }

    /// Try to reserve some [MemoryRegion] based on [Layout] and then return an aligned address (inside the memory pool).
    fn try_reserve(&self, layout: Layout) -> Result<usize, IndexError> {
        let layout = layout.pad_to_align();
        let memory_start = self.memory.get() as usize;

        let mut index = self
            .index
            .try_borrow_mut()
            .map_err(|_| IndexError::IndexAlreadyBorrowed)?;

        let allocation_baker = index.size_region_available(memory_start, layout)?;

        let (region_index, _) = index.split_region(
            allocation_baker.region,
            allocation_baker.offset + layout.size(),
        )?;

        let region = index.get_region_mut(region_index)?;
        region.reserve();

        Ok(region.from + allocation_baker.offset)
    }

    /// Try to free some [MemoryRegion] (here the address is the index in the memory pool).
    fn try_free_addr(&self, addr: usize) -> Result<(), IndexError> {
        let mut index = self
            .index
            .try_borrow_mut()
            .map_err(|_| IndexError::IndexAlreadyBorrowed)?;
        let region_index = index.find_region(addr)?;

        index.get_region_mut(region_index)?.free();
        index.sort_merge();

        Ok(())
    }

    /// Try to perform allocation based on [Layout], internally uses [IndexAllocator::try_reserve] and then perform pointer arithmetic.
    fn try_alloc(&self, layout: Layout) -> Result<*mut u8, IndexError> {
        let offset = self.try_reserve(layout)?;
        Ok(self.memory.get().cast::<u8>().wrapping_add(offset))
    }

    /// Try to free the [MemoryRegion] associated with the pointer given, internally using [IndexAllocator::try_free_addr].
    fn try_free(&self, ptr: *mut u8) -> Result<(), IndexError> {
        let offset = ptr as usize - self.memory.get() as usize;
        self.try_free_addr(offset)?;
        Ok(())
    }

    /// Try to allocate the value in the memory pool and then return a [Box] smart pointer wich manage the memory.
    pub fn try_boxed<T>(&self, val: T) -> Result<Box<T, MEMORY_SIZE, INDEX_SIZE>, IndexError> {
        Box::try_new(val, self)
    }
}

impl<const MEMORY_SIZE: usize, const INDEX_SIZE: usize> Default
    for IndexAllocator<MEMORY_SIZE, INDEX_SIZE>
{
    #[must_use]
    fn default() -> Self {
        Self::empty()
    }
}

unsafe impl<const MEMORY_SIZE: usize, const INDEX_SIZE: usize> GlobalAlloc
    for IndexAllocator<MEMORY_SIZE, INDEX_SIZE>
{
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.try_alloc(layout).unwrap()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        self.try_free(ptr).unwrap();
    }
}

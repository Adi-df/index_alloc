#![no_std]

use core::alloc::{GlobalAlloc, Layout};
use core::cell::{RefCell, UnsafeCell};

pub mod boxed;
mod index;

use boxed::Box;
use index::MemoryIndex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexError {
    NoSuchRegion,
    NoIndexAvailable,
    NoFittingRegion,
    OutOfMemory,
    RegionTooThin,
    EmptyPtr,
}

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
    pub const fn new(memory: [u8; MEMORY_SIZE], index: MemoryIndex<INDEX_SIZE>) -> Self {
        Self {
            memory: UnsafeCell::new(memory),
            index: RefCell::new(index),
        }
    }

    #[must_use]
    pub const fn empty() -> Self {
        Self::new([0; MEMORY_SIZE], MemoryIndex::empty(MEMORY_SIZE))
    }

    fn try_reserve(&self, layout: Layout) -> Result<usize, IndexError> {
        let layout = layout.pad_to_align();
        let memory_start = self.memory.get() as usize;

        let mut index = self.index.borrow_mut();

        let allocation_baker = index.size_region_available(memory_start, layout)?;

        let (region_index, _) = index.split_region(
            allocation_baker.region,
            allocation_baker.offset + layout.size(),
        )?;

        let region = index.get_region_mut(region_index)?;
        region.reserve();

        Ok(region.from + allocation_baker.offset)
    }

    fn try_free_addr(&self, addr: usize) -> Result<(), IndexError> {
        let mut index = self.index.borrow_mut();
        let region_index = index.find_region(addr)?;

        index.get_region_mut(region_index)?.free();
        index.sort_merge();

        Ok(())
    }

    fn try_alloc(&self, layout: Layout) -> Result<*mut u8, IndexError> {
        let offset = self.try_reserve(layout)?;
        Ok(self.memory.get().cast::<u8>().wrapping_add(offset))
    }

    fn try_free(&self, ptr: *mut u8) -> Result<(), IndexError> {
        let offset = ptr as usize - self.memory.get() as usize;
        self.try_free_addr(offset)?;
        Ok(())
    }

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

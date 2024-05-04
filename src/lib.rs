#![no_std]

use core::alloc::{GlobalAlloc, Layout};
use core::cell::{RefCell, UnsafeCell};

pub mod r#box;
mod index;

use index::MemoryIndex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexError {
    NoSuchRegion,
    NoIndexAvailable,
    NoFittingRegion,
    OutOfMemory,
    RegionTooThin,
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
    pub const fn new() -> Self {
        Self {
            memory: UnsafeCell::new([0; MEMORY_SIZE]),
            index: RefCell::new(MemoryIndex::new(MEMORY_SIZE)),
        }
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

    fn try_free(&self, ptr: *mut u8) -> Result<(), IndexError> {
        let offset = ptr as usize - self.memory.get() as usize;
        self.try_free_addr(offset)?;
        Ok(())
    }
}

unsafe impl<const MEMORY_SIZE: usize, const INDEX_SIZE: usize> GlobalAlloc
    for IndexAllocator<MEMORY_SIZE, INDEX_SIZE>
{
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let offset = self.try_reserve(layout).unwrap();
        self.memory.get().cast::<u8>().add(offset)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        self.try_free(ptr).unwrap();
    }
}

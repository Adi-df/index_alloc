#![no_std]

use core::cell::{RefCell, UnsafeCell};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryRegion {
    from: usize,
    size: usize,
    used: bool,
}

impl MemoryRegion {
    #[must_use]
    const fn new(from: usize, size: usize, used: bool) -> Self {
        Self { from, size, used }
    }

    fn reserve(&mut self) {
        self.used = true;
    }

    fn free(&mut self) {
        self.used = false;
    }

    #[must_use]
    fn end(&self) -> usize {
        self.from + self.size
    }

    #[must_use]
    fn contains(&self, addr: usize) -> bool {
        self.from <= addr && addr < self.from + self.size
    }
}

#[derive(Debug, Clone)]
pub struct MemoryIndex<const INDEX_SIZE: usize> {
    regions: [Option<MemoryRegion>; INDEX_SIZE],
    sorted: bool,
}

impl<const INDEX_SIZE: usize> MemoryIndex<INDEX_SIZE> {
    const fn new(memory_size: usize) -> Self {
        const NONE: Option<MemoryRegion> = None;
        let mut regions = [NONE; INDEX_SIZE];
        regions[0] = Some(MemoryRegion::new(0, memory_size, false));

        Self {
            regions,
            sorted: true,
        }
    }
}

pub struct IndexAllocator<const MEMORY_SIZE: usize, const INDEX_SIZE: usize> {
    memory: UnsafeCell<[u8; MEMORY_SIZE]>,
    index: RefCell<MemoryIndex<INDEX_SIZE>>,
}

impl<const MEMORY_SIZE: usize, const INDEX_SIZE: usize> IndexAllocator<MEMORY_SIZE, INDEX_SIZE> {
    pub const fn new() -> Self {
        Self {
            memory: UnsafeCell::new([0; MEMORY_SIZE]),
            index: RefCell::new(MemoryIndex::new(MEMORY_SIZE)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}

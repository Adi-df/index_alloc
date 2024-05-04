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

pub struct IndexAllocator<const MEMORY_SIZE: usize, const INDEX_SIZE: usize> {
    memory: UnsafeCell<[u8; MEMORY_SIZE]>,
    index: RefCell<MemoryIndex<INDEX_SIZE>>,
}

#[cfg(test)]
mod tests {
    use super::*;
}

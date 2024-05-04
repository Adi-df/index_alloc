#![no_std]

use core::cell::{RefCell, UnsafeCell};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexError {
    NoSuchRegion,
    NoIndexAvailable,
    NoFittingRegion,
    OutOfMemory,
    RegionTooThin,
}

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
}

impl<const INDEX_SIZE: usize> MemoryIndex<INDEX_SIZE> {
    const fn new(memory_size: usize) -> Self {
        const NONE: Option<MemoryRegion> = None;
        let mut regions = [NONE; INDEX_SIZE];
        regions[0] = Some(MemoryRegion::new(0, memory_size, false));

        Self { regions }
    }

    fn get_region(&self, region: usize) -> Result<&MemoryRegion, IndexError> {
        self.regions[region]
            .as_ref()
            .ok_or(IndexError::NoSuchRegion)
    }

    fn get_region_mut(&mut self, region: usize) -> Result<&mut MemoryRegion, IndexError> {
        self.regions[region]
            .as_mut()
            .ok_or(IndexError::NoSuchRegion)
    }

    fn available_index(&self) -> Result<usize, IndexError> {
        self.regions
            .iter()
            .enumerate()
            .find_map(|(i, maybe_region)| {
                if maybe_region.is_none() {
                    Some(i)
                } else {
                    None
                }
            })
            .ok_or(IndexError::NoIndexAvailable)
    }

    fn size_region_available(&self, size: usize) -> Result<usize, IndexError> {
        self.regions
            .iter()
            .enumerate()
            .find_map(|(i, maybe_region)| match maybe_region {
                Some(region) if region.size >= size && !region.used => Some(i),
                _ => None,
            })
            .ok_or(IndexError::NoFittingRegion)
    }

    fn find_region(&self, addr: usize) -> Result<usize, IndexError> {
        self.regions
            .iter()
            .enumerate()
            .find_map(|(i, maybe_region)| match maybe_region {
                Some(region) if region.contains(addr) => Some(i),
                _ => None,
            })
            .ok_or(IndexError::OutOfMemory)
    }

    fn split_region(&mut self, region: usize, size: usize) -> Result<(usize, usize), IndexError> {
        if self.get_region(region)?.size < size {
            return Err(IndexError::RegionTooThin);
        }

        let right_index = self.available_index()?;
        let left_region = self.get_region_mut(region)?;

        let left_size = size;
        let right_size = left_region.size - size;

        left_region.size = left_size;
        self.regions[right_index] = Some(MemoryRegion::new(
            left_region.end(),
            right_size,
            left_region.used,
        ));

        Ok((region, right_index))
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

    fn create_index<const INDEX_SIZE: usize>(
        size: usize,
        from: &[Option<MemoryRegion>],
    ) -> MemoryIndex<INDEX_SIZE> {
        let mut index = MemoryIndex::new(size);
        for (i, region) in from.iter().enumerate() {
            index.regions[i] = region.clone();
        }
        index
    }

    #[test]
    fn test_available_index() {
        let index: MemoryIndex<8> = create_index(
            64,
            &[
                Some(MemoryRegion::new(0, 16, false)),
                Some(MemoryRegion::new(16, 16, true)),
                None,
                Some(MemoryRegion::new(32, 32, false)),
            ],
        );

        assert_eq!(index.available_index(), Ok(2));

        let index: MemoryIndex<4> = create_index(
            64,
            &[
                Some(MemoryRegion::new(0, 16, false)),
                Some(MemoryRegion::new(16, 16, true)),
                Some(MemoryRegion::new(32, 16, false)),
                Some(MemoryRegion::new(48, 16, false)),
            ],
        );

        assert_eq!(index.available_index(), Err(IndexError::NoIndexAvailable));
    }

    #[test]
    fn test_index_size_region_available() {
        let index: MemoryIndex<8> = create_index(
            64,
            &[
                Some(MemoryRegion::new(0, 8, false)),
                Some(MemoryRegion::new(8, 32, true)),
                Some(MemoryRegion::new(40, 16, false)),
                Some(MemoryRegion::new(56, 8, false)),
            ],
        );

        assert_eq!(index.size_region_available(16), Ok(2));
        assert_eq!(
            index.size_region_available(32),
            Err(IndexError::NoFittingRegion)
        );
    }
}

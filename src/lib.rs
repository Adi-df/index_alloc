#![no_std]

use core::alloc::{GlobalAlloc, Layout};
use core::cell::{RefCell, UnsafeCell};
use core::cmp::Ordering;

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

    fn sort_merge(&mut self) {
        self.regions
            .sort_unstable_by(|region1, region2| match (region1, region2) {
                (Some(r1), Some(r2)) => r1.from.cmp(&r2.from),
                (None, Some(_)) => Ordering::Greater,
                (Some(_), None) => Ordering::Less,
                (None, None) => Ordering::Equal,
            });

        let mut new_counter = 0;
        let mut counter = 0;

        while let Some(region) = &self.regions[counter] {
            if region.used {
                self.regions[new_counter] = Some(region.clone());
                new_counter += 1;
                counter += 1;
            } else {
                let from = region.from;
                let mut size = 0;

                for i in counter..INDEX_SIZE {
                    if let Some(r) = &self.regions[i] {
                        if r.used {
                            self.regions[new_counter] = Some(MemoryRegion::new(from, size, false));
                            new_counter += 1;
                            counter = i;
                            break;
                        } else {
                            size += r.size;
                        }
                    } else {
                        self.regions[new_counter] = Some(MemoryRegion::new(from, size, false));
                        new_counter += 1;
                        counter = i;
                        break;
                    }
                }
            }
        }

        for i in new_counter..INDEX_SIZE {
            self.regions[i] = None;
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

    fn try_reserve(&self, layout: Layout) -> Result<usize, IndexError> {
        let mut index = self.index.borrow_mut();

        let mem_size = (layout.size() / layout.align() + 1) * layout.align();

        let spliting_region = index.size_region_available(mem_size)?;
        let (region_index, _) = index.split_region(spliting_region, mem_size)?;

        let region = index.get_region_mut(region_index)?;
        region.reserve();

        Ok(region.from)
    }

    fn try_free(&self, addr: usize) -> Result<(), IndexError> {
        let mut index = self.index.borrow_mut();
        let region_index = index.find_region(addr)?;

        index.get_region_mut(region_index)?.free();
        index.sort_merge();

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
        let offset = ptr as usize - self.memory.get() as usize;
        self.try_free(offset).unwrap();
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

    #[test]
    fn test_split_region() {
        let mut index: MemoryIndex<8> = create_index(
            64,
            &[
                Some(MemoryRegion::new(0, 8, false)),
                Some(MemoryRegion::new(8, 32, true)),
                Some(MemoryRegion::new(40, 16, false)),
                Some(MemoryRegion::new(56, 8, false)),
            ],
        );

        assert_eq!(index.split_region(2, 8), Ok((2, 4)));

        assert_eq!(
            *index.get_region(2).unwrap(),
            MemoryRegion::new(40, 8, false)
        );
        assert_eq!(
            *index.get_region(4).unwrap(),
            MemoryRegion::new(48, 8, false)
        );

        assert_eq!(index.split_region(0, 16), Err(IndexError::RegionTooThin));
    }

    #[test]
    fn test_index_sort() {
        let index_blueprint = [
            Some(MemoryRegion::new(0, 16, false)),
            None,
            Some(MemoryRegion::new(32, 16, false)),
            Some(MemoryRegion::new(48, 16, true)),
            None,
            Some(MemoryRegion::new(16, 16, true)),
        ];
        let mut index: MemoryIndex<8> = create_index(64, &index_blueprint);

        index.sort_merge();

        assert_eq!(
            index.get_region(0).unwrap(),
            index_blueprint[0].as_ref().unwrap()
        );
        assert_eq!(
            index.get_region(1).unwrap(),
            index_blueprint[5].as_ref().unwrap()
        );
        assert_eq!(
            index.get_region(2).unwrap(),
            index_blueprint[2].as_ref().unwrap()
        );
        assert_eq!(
            index.get_region(3).unwrap(),
            index_blueprint[3].as_ref().unwrap()
        );
    }

    #[test]
    fn test_index_merge() {
        let index_blueprint = [
            Some(MemoryRegion::new(0, 16, false)),
            None,
            Some(MemoryRegion::new(32, 16, true)),
            Some(MemoryRegion::new(48, 16, true)),
            None,
            Some(MemoryRegion::new(16, 16, false)),
        ];
        let mut index: MemoryIndex<8> = create_index(64, &index_blueprint);

        index.sort_merge();

        assert_eq!(
            *index.get_region(0).unwrap(),
            MemoryRegion::new(0, 32, false)
        );
        assert_eq!(
            index.get_region(1).unwrap(),
            index_blueprint[2].as_ref().unwrap()
        );
        assert_eq!(
            index.get_region(2).unwrap(),
            index_blueprint[3].as_ref().unwrap()
        );
    }
}

use core::alloc::Layout;
use core::cmp::Ordering;

use crate::IndexError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryRegion {
    pub from: usize,
    pub size: usize,
    pub used: bool,
}

impl MemoryRegion {
    #[must_use]
    pub const fn new(from: usize, size: usize, used: bool) -> Self {
        Self { from, size, used }
    }

    pub fn reserve(&mut self) {
        self.used = true;
    }

    pub fn free(&mut self) {
        self.used = false;
    }

    #[must_use]
    pub fn end(&self) -> usize {
        self.from + self.size
    }

    #[must_use]
    pub fn contains(&self, addr: usize) -> bool {
        self.from <= addr && addr < self.from + self.size
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllocationBaker {
    pub region: usize,
    pub offset: usize,
}

#[derive(Debug, Clone)]
pub struct MemoryIndex<const INDEX_SIZE: usize> {
    regions: [Option<MemoryRegion>; INDEX_SIZE],
}

impl<const INDEX_SIZE: usize> MemoryIndex<INDEX_SIZE> {
    pub const fn new(regions: [Option<MemoryRegion>; INDEX_SIZE]) -> Self {
        Self { regions }
    }

    pub const fn empty(memory_size: usize) -> Self {
        const NONE: Option<MemoryRegion> = None;
        let mut regions = [NONE; INDEX_SIZE];
        regions[0] = Some(MemoryRegion::new(0, memory_size, false));
        Self::new(regions)
    }

    pub fn get_region(&self, region: usize) -> Result<&MemoryRegion, IndexError> {
        self.regions[region]
            .as_ref()
            .ok_or(IndexError::NoSuchRegion)
    }

    pub fn get_region_mut(&mut self, region: usize) -> Result<&mut MemoryRegion, IndexError> {
        self.regions[region]
            .as_mut()
            .ok_or(IndexError::NoSuchRegion)
    }

    pub fn available_index(&self) -> Result<usize, IndexError> {
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

    pub fn find_region(&self, addr: usize) -> Result<usize, IndexError> {
        self.regions
            .iter()
            .enumerate()
            .find_map(|(i, maybe_region)| match maybe_region {
                Some(region) if region.contains(addr) => Some(i),
                _ => None,
            })
            .ok_or(IndexError::OutOfMemory)
    }

    pub fn size_region_available(
        &self,
        memory_start: usize,
        layout: Layout,
    ) -> Result<AllocationBaker, IndexError> {
        self.regions
            .iter()
            .enumerate()
            .find_map(|(i, maybe_region)| match maybe_region {
                Some(region) if !region.used => {
                    let offset = (memory_start + region.from).next_multiple_of(layout.align())
                        - memory_start
                        - region.from;
                    if region.from + offset + layout.size() <= region.end() {
                        Some(AllocationBaker { region: i, offset })
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .ok_or(IndexError::NoFittingRegion)
    }

    pub fn split_region(
        &mut self,
        region: usize,
        size: usize,
    ) -> Result<(usize, usize), IndexError> {
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

    pub fn sort_merge(&mut self) {
        self.regions
            .sort_unstable_by(|region1, region2| match (region1, region2) {
                (Some(r1), Some(r2)) => r1.from.cmp(&r2.from),
                (None, Some(_)) => Ordering::Greater,
                (Some(_), None) => Ordering::Less,
                (None, None) => Ordering::Equal,
            });

        let mut new_counter = 0;
        let mut counter = 0;

        'merge_loop: while let Some(region) = &self.regions[counter] {
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

                            if i + 1 == INDEX_SIZE {
                                self.regions[new_counter] =
                                    Some(MemoryRegion::new(from, size, false));
                                break 'merge_loop;
                            }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_index<const INDEX_SIZE: usize>(
        size: usize,
        from: &[Option<MemoryRegion>],
    ) -> MemoryIndex<INDEX_SIZE> {
        let mut index = MemoryIndex::empty(size);
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
            128,
            &[
                Some(MemoryRegion::new(0, 8, false)),
                Some(MemoryRegion::new(8, 32, true)),
                Some(MemoryRegion::new(40, 16, false)),
                Some(MemoryRegion::new(56, 32, true)),
                Some(MemoryRegion::new(88, 32, false)),
                Some(MemoryRegion::new(120, 8, false)),
            ],
        );

        assert_eq!(
            index.size_region_available(0, Layout::from_size_align(16, 1).unwrap()),
            Ok(AllocationBaker {
                region: 2,
                offset: 0
            })
        );
        assert_eq!(
            index.size_region_available(0, Layout::from_size_align(64, 1).unwrap()),
            Err(IndexError::NoFittingRegion)
        );
        assert_eq!(
            index.size_region_available(0, Layout::from_size_align(16, 16).unwrap()),
            Ok(AllocationBaker {
                region: 4,
                offset: 8
            })
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

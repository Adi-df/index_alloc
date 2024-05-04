use core::alloc::Layout;
use core::cmp::Ordering;

use crate::IndexError;

/// The representation of a region of the memory pool in the index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryRegion {
    pub from: usize,
    pub size: usize,
    pub used: bool,
}

impl MemoryRegion {
    /// Create a new [`MemoryRegion`].
    #[must_use]
    pub const fn new(from: usize, size: usize, used: bool) -> Self {
        Self { from, size, used }
    }

    /// Mark the region as used.
    pub fn reserve(&mut self) {
        self.used = true;
    }

    /// Mark the region as available for use.
    pub fn free(&mut self) {
        self.used = false;
    }

    /// Compute the end address of the region.
    #[must_use]
    pub fn end(&self) -> usize {
        self.from + self.size
    }

    /// Test if the region contains the specified address.
    #[must_use]
    pub fn contains(&self, addr: usize) -> bool {
        self.from <= addr && addr < self.from + self.size
    }
}

/// The representation of a region prepared to allocate a layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllocationBaker {
    /// The region in wich the allocation needs to be performed.
    pub region: usize,
    /// The offset needed in order for the pointer to be correctly aligned.
    pub offset: usize,
}

/// The type storing the memroy regions informations and so keeping the abstract representation of the memory pool.
#[derive(Debug, Clone)]
pub struct MemoryIndex<const INDEX_SIZE: usize> {
    regions: [Option<MemoryRegion>; INDEX_SIZE],
}

impl<const INDEX_SIZE: usize> MemoryIndex<INDEX_SIZE> {
    /// Create the [`MemoryIndex`] based on preexisting partition.
    pub const fn new(regions: [Option<MemoryRegion>; INDEX_SIZE]) -> Self {
        Self { regions }
    }

    /// Create the [`MemoryIndex`] as a single region containing the whole memory pool.
    pub const fn empty(memory_size: usize) -> Self {
        const NONE: Option<MemoryRegion> = None;
        let mut regions = [NONE; INDEX_SIZE];
        regions[0] = Some(MemoryRegion::new(0, memory_size, false));
        Self::new(regions)
    }

    /// Get the region at the specified index.
    /// Raise an [`IndexError::NoSuchRegion`] if the index is not a region.
    pub fn get_region(&self, region: usize) -> Result<&MemoryRegion, IndexError> {
        self.regions[region]
            .as_ref()
            .ok_or(IndexError::NoSuchRegion)
    }

    /// Get mutable access the region at the specified index.
    /// Raise an [`IndexError::NoSuchRegion`] if the index is not a region.
    pub fn get_region_mut(&mut self, region: usize) -> Result<&mut MemoryRegion, IndexError> {
        self.regions[region]
            .as_mut()
            .ok_or(IndexError::NoSuchRegion)
    }

    /// Get an index corresponding to an empty index.
    /// Raise an [`IndexError::NoIndexAvailable`] if the index is full.
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

    /// Find the region corresponding with the given address (where the address is relative to the memory pool).
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

    /// Look for a memory region ready to store data corresponding to a certain [Layout].
    /// Raise an [`Index::NoFittingRegion`] if no region satisfy the [Layout] needs.
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

    /// Split a region in two based on size to prepare for allocation.
    /// Return a couple of region index corresponding to the left and right parts of the cut.
    /// The left region is set to have the desired size.
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

    /// Sort region index in ascending order and then merge continuous, non-allocated regions.
    pub fn sort_merge(&mut self) {
        self.regions
            .sort_unstable_by(|region1, region2| match (region1, region2) {
                (Some(r1), Some(r2)) => r1.from.cmp(&r2.from),
                (None, Some(_)) => Ordering::Greater,
                (Some(_), None) => Ordering::Less,
                (None, None) => Ordering::Equal,
            });

        // The merging process look for non-allocated continuous ranges and group them in single [MemoryRegion].

        // [new_counter] and [counter] are like to pointers to elements of the region index.
        // [new_counter] overwrite the index whereas [counter] reads it.
        // The new position of the region.
        let mut new_counter = 0;
        // The current region being processed.
        let mut counter = 0;

        // Loop through the index while it represents regions.
        'merge_loop: while let Some(region) = &self.regions[counter] {
            if region.used {
                // If the region is used, let in place.
                self.regions[new_counter] = Some(region.clone());
                new_counter += 1;
                counter += 1;
            } else {
                // Keep in track where the new merged region start and it's new size.
                let from = region.from;
                let mut size = 0;

                // Walkthrough the rest of the index until:
                // - It's the end of the index (or it's full), in that case, stop the whole process.
                // - The next region is used, in that case merge the current regions and continue to the next one.
                for i in counter..INDEX_SIZE {
                    if let Some(r) = &self.regions[i] {
                        if r.used {
                            // If it's use, merge everything and go on.
                            self.regions[new_counter] = Some(MemoryRegion::new(from, size, false));
                            new_counter += 1;
                            counter = i;
                            break;
                        } else {
                            // If it's not, add the size of the current region to the size counter.
                            size += r.size;

                            if i + 1 == INDEX_SIZE {
                                // If the index is full stop the whole process.
                                self.regions[new_counter] =
                                    Some(MemoryRegion::new(from, size, false));
                                break 'merge_loop;
                            }
                        }
                    } else {
                        // If it's not a region, stop the merging process.
                        self.regions[new_counter] = Some(MemoryRegion::new(from, size, false));
                        new_counter += 1;
                        counter = i;
                        break;
                    }
                }
            }
        }

        // After all the part that could merged was, earase the remaining free indexes.
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

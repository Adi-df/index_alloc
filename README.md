# Index Alloc
A simple, toy static Allocator wich use a fixed length array to store allocated data.

This crate expose a struct [`IndexAllocator`] wich implement [`GlobalAlloc`] so it can be uses as the global allocator in `no_std` environement.

Disadvantages :
- Extremly unsafe
- Very slow
- Memory inefficient

Even though it seems unusable, it has plenty of advantages :
- Just joking don't use that

To store allocated memory, [`IndexAllocator`] uses a `MemoryIndex` wich stores a list of regions containing the state of the region (size, from which address, used status). For instance :

```rust
use index_alloc::IndexAllocator;

#[global_allocator]
static ALLOCATOR: IndexAllocator<2048, 16> = IndexAllocator::empty();

fn main() {
    let test_str = String::from("Hello World");
    println!("{test_str}");
}
```

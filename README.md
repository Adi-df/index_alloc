# Index Alloc
A simple, toy static Allocator which use a fixed length array to store allocated data.

This crate expose a struct [`IndexAllocator`] which implement [`GlobalAlloc`] so it can be uses as the global allocator in `no_std` environment.

Disadvantages :
- Extremely unsafe
- Very slow
- Memory inefficient

Even though it seems unusable, it has plenty of advantages :
- Just kidding, don't use that

To store allocated memory, [`IndexAllocator`] uses a `MemoryIndex` which stores a list of regions containing the state of the region (size, from which address, used status). For instance :

```rust
use index_alloc::IndexAllocator;

#[global_allocator]
static ALLOCATOR: IndexAllocator<2048, 16> = IndexAllocator::empty();

fn main() {
    let test_str = String::from("Hello World");
    println!("{test_str}");
}
```

See more example in the [`Repository's Examples`].

[`Repository's Examples`]: https://github.com/Adi-df/index_alloc/tree/master/examples

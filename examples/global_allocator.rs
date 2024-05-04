use index_alloc::IndexAllocator;

#[global_allocator]
static ALLOCATOR: IndexAllocator<2048, 32> = IndexAllocator::new();

fn main() {
    let mut test_str = String::from("Hello World!\n");
    test_str.push_str("This is an example of a String allocated in IndexAllocator");

    println!("{test_str}");

    let test_vec: Vec<String> = (0..=10)
        .into_iter()
        .map(|i| format!("Number {i}"))
        .collect();

    println!("{:?}", test_vec);
}

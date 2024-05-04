use index_alloc::boxed::Box;
use index_alloc::{IndexAllocator, IndexError};

pub struct ListIterator<'a, 'b, T, const MEMORY_SIZE: usize, const INDEX_SIZE: usize>
where
    'a: 'b,
{
    list: &'b List<'a, T, MEMORY_SIZE, INDEX_SIZE>,
}

impl<'a, 'b, T, const MEMORY_SIZE: usize, const INDEX_SIZE: usize> Iterator
    for ListIterator<'a, 'b, T, MEMORY_SIZE, INDEX_SIZE>
where
    'a: 'b,
{
    type Item = &'b T;

    fn next(&mut self) -> Option<Self::Item> {
        match self.list {
            List::Cons(val, next) => {
                self.list = &*next;
                Some(val)
            }
            List::Nil(_) => None,
        }
    }
}

pub enum List<'a, T, const MEMORY_SIZE: usize, const INDEX_SIZE: usize> {
    Cons(
        T,
        Box<'a, List<'a, T, MEMORY_SIZE, INDEX_SIZE>, MEMORY_SIZE, INDEX_SIZE>,
    ),
    Nil(&'a IndexAllocator<MEMORY_SIZE, INDEX_SIZE>),
}

impl<'a, T, const MEMORY_SIZE: usize, const INDEX_SIZE: usize>
    List<'a, T, MEMORY_SIZE, INDEX_SIZE>
{
    pub fn empty(allocator: &'a IndexAllocator<MEMORY_SIZE, INDEX_SIZE>) -> Self {
        Self::Nil(allocator)
    }

    pub fn push(&mut self, val: T) -> Result<(), IndexError> {
        match self {
            Self::Nil(allocator) => {
                *self = Self::Cons(val, allocator.try_boxed(Self::Nil(allocator))?);
                Ok(())
            }
            Self::Cons(_, list) => list.push(val),
        }
    }
}

impl<'a, 'b, T, const MEMORY_SIZE: usize, const INDEX_SIZE: usize> IntoIterator
    for &'b List<'a, T, MEMORY_SIZE, INDEX_SIZE>
where
    'a: 'b,
{
    type IntoIter = ListIterator<'a, 'b, T, MEMORY_SIZE, INDEX_SIZE>;
    type Item = &'b T;
    fn into_iter(self) -> Self::IntoIter {
        ListIterator { list: self }
    }
}

fn main() {
    let allocator: IndexAllocator<128, 8> = IndexAllocator::empty();

    let mut list = List::empty(&allocator);
    list.push(1).unwrap();
    list.push(2).unwrap();
    list.push(3).unwrap();
    list.push(4).unwrap();

    {
        for el in &list {
            println!("{el}");
        }
    }
}

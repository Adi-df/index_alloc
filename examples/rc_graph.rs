use index_alloc::rc::{Rc, Weak};
use index_alloc::IndexAllocator;

const MEMORY_SIZE: usize = 1024;
const INDEX_SIZE: usize = 64;

#[derive(Clone)]
pub enum Link<'a, T> {
    Held(Rc<'a, Node<'a, T>, MEMORY_SIZE, INDEX_SIZE>),
    Weak(Weak<'a, Node<'a, T>, MEMORY_SIZE, INDEX_SIZE>),
}

impl<'a, T> Link<'a, T> {
    fn new_held(node: Rc<'a, Node<'a, T>, MEMORY_SIZE, INDEX_SIZE>) -> Self {
        Self::Held(node)
    }

    fn new_weak(node: Weak<'a, Node<'a, T>, MEMORY_SIZE, INDEX_SIZE>) -> Self {
        Self::Weak(node)
    }

    fn to(&self) -> Option<Rc<'a, Node<'a, T>, MEMORY_SIZE, INDEX_SIZE>> {
        match self {
            Self::Held(rc) => Some(rc.clone()),
            Self::Weak(weak) => weak.upgrade(),
        }
    }
}

pub struct Node<'a, T> {
    val: T,
    links: [Option<Link<'a, T>>; 4],
}

impl<'a, T> Node<'a, T> {
    fn new(val: T) -> Self {
        Self {
            val,
            links: [None, None, None, None],
        }
    }

    fn add_link(&mut self, link: Link<'a, T>) {
        for l in self.links.iter_mut() {
            if matches!(l, None) {
                *l = Some(link);
                return;
            }
        }
        panic!("No more links available");
    }

    fn iter_links(&self) -> impl Iterator<Item = &Link<'a, T>> {
        self.links.iter().filter_map(move |l| l.as_ref())
    }
}

fn main() {
    let allocator: IndexAllocator<1024, 64> = IndexAllocator::empty();

    let mut main_node = Node::new("Main");
    let first_child_node = Rc::try_new(Node::new("First child node"), &allocator).unwrap();

    let second_child_node = {
        let mut node = Node::new("Second child node");
        node.add_link(Link::Weak(first_child_node.downgrade()));
        Rc::try_new(node, &allocator).unwrap()
    };

    main_node.add_link(Link::new_held(first_child_node.clone()));
    main_node.add_link(Link::new_held(second_child_node.clone()));

    for l in main_node.iter_links() {
        println!("Main node is linked to : {}", l.to().unwrap().val);
    }

    for l in second_child_node.iter_links() {
        println!("Second child node is linked to : {}", l.to().unwrap().val);
    }
}

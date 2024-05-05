use index_alloc::boxed::Box;
use index_alloc::IndexAllocator;

const MEMORY_SIZE: usize = 1024;
const INDEX_SIZE: usize = 32;

static ALLOCATOR: IndexAllocator<MEMORY_SIZE, INDEX_SIZE> = IndexAllocator::empty();

pub type BoxedListener<'a> = Box<'a, dyn Listener + 'static, MEMORY_SIZE, INDEX_SIZE>;

pub struct EventDispatcher<'a, const N: usize> {
    allocator: &'a IndexAllocator<MEMORY_SIZE, INDEX_SIZE>,
    listeners: [Option<BoxedListener<'a>>; N],
    counter: usize,
}

impl<'a, const N: usize> EventDispatcher<'a, N> {
    pub fn empty(allocator: &'a IndexAllocator<MEMORY_SIZE, INDEX_SIZE>) -> Self {
        const NONE: Option<BoxedListener> = None;
        Self {
            allocator,
            listeners: [NONE; N],
            counter: 0,
        }
    }

    pub fn register<T>(&mut self, listener: T)
    where
        T: Listener + 'static,
        &'a mut (dyn Listener + 'static): From<&'a mut T>,
    {
        if self.counter >= N {
            panic!("Out of listeners");
        }

        self.listeners[self.counter] = Some(
            self.allocator
                .try_boxed::<'a, dyn Listener + 'static, T>(listener)
                .unwrap(),
        );
        self.counter += 1;
    }

    pub fn send(&mut self, name: &str) {
        for listener in self.listeners.iter_mut() {
            if let Some(l) = listener {
                l.on_event(name);
            }
        }
    }
}

pub trait Listener {
    fn on_event(&mut self, name: &str);
}

impl<'a, T: Listener> From<&'a mut T> for &'a mut dyn Listener {
    fn from(value: &'a mut T) -> Self {
        value as _
    }
}

pub struct DummyListener;

impl Listener for DummyListener {
    fn on_event(&mut self, name: &str) {
        println!("Received : {name}");
    }
}

fn main() {
    let mut dispatcher: EventDispatcher<16> = EventDispatcher::empty(&ALLOCATOR);

    let dummy = DummyListener;
    dispatcher.register(dummy);
    dispatcher.send("It's a test");
}

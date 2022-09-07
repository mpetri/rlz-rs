use parking_lot::Mutex;
use std::sync::Arc;

const DEFAULT_CAPACITY: usize = 16384;

#[derive(Clone, Debug)]
pub(crate) struct Scratch {
    pub(crate) literals: Vec<u8>,
    pub(crate) offsets: Vec<u32>,
    pub(crate) lens: Vec<u32>,
}

impl Scratch {
    pub fn clear(&mut self) {
        self.literals.clear();
        self.offsets.clear();
        self.lens.clear();
    }
}

impl Default for Scratch {
    fn default() -> Scratch {
        Scratch {
            literals: Vec::with_capacity(DEFAULT_CAPACITY),
            offsets: Vec::with_capacity(DEFAULT_CAPACITY),
            lens: Vec::with_capacity(DEFAULT_CAPACITY),
        }
    }
}

pub(crate) struct ScratchSpace {
    available: Arc<Mutex<Vec<Scratch>>>,
}

impl Default for ScratchSpace {
    fn default() -> ScratchSpace {
        ScratchSpace {
            available: Arc::new(Mutex::new(vec![Scratch::default(); 256])),
        }
    }
}

impl ScratchSpace {
    pub(crate) fn get(&self) -> Scratch {
        self.available.lock().pop().unwrap_or_default()
    }

    pub(crate) fn release(&self, scratch: Scratch) {
        self.available.lock().push(scratch)
    }
}

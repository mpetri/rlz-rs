use bytes::BytesMut;
use parking_lot::Mutex;
use std::sync::Arc;

const DEFAULT_CAPACITY: usize = 1024 * 4096;

#[derive(Clone, Debug)]
pub(crate) struct Scratch {
    pub(crate) encoded: BytesMut,
    pub(crate) literals: BytesMut,
    pub(crate) offsets: BytesMut,
    pub(crate) lens: BytesMut,
}

impl Scratch {
    pub fn clear(&mut self) {
        self.encoded.clear();
        self.literals.clear();
        self.offsets.clear();
        self.lens.clear();
    }

    pub fn reserve_encoded(&mut self, bytes: usize) {
        let bytes = 1024 + bytes * 5; // some safety here..
        self.encoded.clear();
        self.encoded.reserve(bytes);
        unsafe {
            self.encoded.set_len(bytes);
        }
    }

    pub fn reserve_output(&mut self, bytes: usize) {
        let bytes = 1024 + bytes * 5; // some safety here..
        self.literals.clear();
        self.literals.reserve(bytes);
        self.offsets.clear();
        self.offsets.reserve(bytes);
        self.lens.clear();
        self.lens.reserve(bytes);
        unsafe {
            self.literals.set_len(bytes);
            self.offsets.set_len(bytes);
            self.lens.set_len(bytes);
        }
    }
}

impl Default for Scratch {
    fn default() -> Scratch {
        Scratch {
            encoded: BytesMut::with_capacity(DEFAULT_CAPACITY),
            literals: BytesMut::with_capacity(DEFAULT_CAPACITY),
            offsets: BytesMut::with_capacity(DEFAULT_CAPACITY),
            lens: BytesMut::with_capacity(DEFAULT_CAPACITY),
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

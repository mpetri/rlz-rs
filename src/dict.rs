use std::ops::Deref;

use bytes::Bytes;
mod reservoir;

use reservoir::Reservoir;

#[derive(Clone)]
pub struct Dictionary(Bytes);

impl Dictionary {
    pub fn builder(dict_mib: usize, sample_size: usize, reservoir_mib: usize) -> DictionaryBuilder {
        DictionaryBuilder::new(dict_mib, sample_size, reservoir_mib)
    }

    pub fn from(mut bytes: impl bytes::Buf) -> Self {
        Self(bytes.copy_to_bytes(bytes.remaining()))
    }
}

#[derive(Default)]
pub struct DictionaryBuilder {
    dict_size: usize,
    reservoir: Reservoir,
}

impl DictionaryBuilder {
    pub fn new(dict_mib: usize, sample_size: usize, reservoir_mib: usize) -> Self {
        let reservoir_size = (reservoir_mib * 1024 * 1024) / sample_size;
        Self {
            dict_size: dict_mib * 1024 * 1024,
            reservoir: Reservoir::empty(sample_size, reservoir_size),
        }
    }

    #[tracing::instrument(skip_all)]
    pub fn sample(&mut self, bytes: &[u8]) {
        self.reservoir.maybe_add(bytes);
    }

    #[tracing::instrument(skip_all)]
    pub fn finish(self) -> Dictionary {
        Dictionary(self.reservoir.freeze(self.dict_size))
    }
}

impl Deref for Dictionary {
    type Target = Bytes;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

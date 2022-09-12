use bytes::{BufMut, Bytes, BytesMut};
use rand::seq::SliceRandom;
use rand::Rng;

/// reservoir based dictionary construction
#[derive(Default)]
pub struct ReservoirDictionaryBuilder {
    dict_size: usize,
    sample_size: usize,
    itr: usize,
    samples: Vec<Option<Bytes>>,
}

impl ReservoirDictionaryBuilder {
    #[tracing::instrument(skip_all)]
    pub(crate) fn empty(dict_mib: usize, sample_size: usize, reservoir_mib: usize) -> Self {
        let reservoir_size = (reservoir_mib * 1024 * 1024) / sample_size;
        Self {
            dict_size: dict_mib * 1024 * 1024,
            sample_size,
            itr: reservoir_size,
            samples: vec![None; reservoir_size],
        }
    }

    #[tracing::instrument(skip_all)]
    pub(crate) fn freeze(mut self, size_in_bytes: usize) -> Bytes {
        let mut rng = rand::thread_rng();
        self.samples.shuffle(&mut rng);
        let mut final_dict = BytesMut::with_capacity(size_in_bytes);
        for sample in self.samples.into_iter().filter_map(|l| l) {
            final_dict.put_slice(&sample);
            if final_dict.len() == size_in_bytes {
                break;
            }
        }
        final_dict.freeze()
    }

    /// finish dictionary construction and create dictionary
    #[tracing::instrument(skip_all)]
    pub fn finish(self) -> super::Dictionary {
        let dict_size = self.dict_size;
        super::Dictionary(self.freeze(dict_size))
    }

    /// sample from a slice of new bytes
    #[tracing::instrument(skip_all)]
    pub fn sample(&mut self, new_bytes: &[u8]) {
        let mut rng = rand::thread_rng();
        for sample in new_bytes.chunks(self.sample_size) {
            let random_number = rng.gen_range(0..self.itr);
            if random_number < self.samples.len() {
                self.samples[random_number] = Some(Bytes::copy_from_slice(sample));
            }
            self.itr += 1;
        }
    }
}

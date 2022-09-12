use bytes::{BufMut, Bytes, BytesMut};
use rand::Rng;
use std::collections::HashMap;
use std::hash::Hasher;

#[derive(Default)]
pub struct StratifiedReservoirDictionaryBuilder {
    dict_size: usize,
    sample_size: usize,
    items_per_bucket: usize,
    itr: HashMap<u64, usize>,
    samples: HashMap<u64, Vec<Option<Bytes>>>,
}

impl StratifiedReservoirDictionaryBuilder {
    #[tracing::instrument(skip_all)]
    pub(crate) fn empty(dict_mib: usize, sample_size: usize, items_per_bucket: usize) -> Self {
        Self {
            dict_size: dict_mib * 1024 * 1024,
            sample_size,
            items_per_bucket,
            itr: HashMap::new(),
            samples: HashMap::new(),
        }
    }

    #[tracing::instrument(skip_all)]
    pub(crate) fn freeze(self, size_in_bytes: usize) -> Bytes {
        let num_buckets = self.samples.len();
        let num_samples = self.dict_size / self.sample_size;
        let samples_per_bucket = (num_samples / num_buckets).max(1);
        let mut final_dict = BytesMut::with_capacity(size_in_bytes);
        for reservoir in self.samples.into_values() {
            for sample in reservoir
                .into_iter()
                .filter_map(|l| l)
                .take(samples_per_bucket)
            {
                final_dict.put_slice(&sample);
                if final_dict.len() == size_in_bytes {
                    break;
                }
            }
        }
        final_dict.freeze()
    }

    #[tracing::instrument(skip_all)]
    pub fn finish(self) -> super::Dictionary {
        let dict_size = self.dict_size;
        super::Dictionary(self.freeze(dict_size))
    }

    #[tracing::instrument(skip_all)]
    pub fn sample(&mut self, identifier: impl std::hash::Hash, new_bytes: &[u8]) {
        let mut hasher = metrohash::MetroHash64::new();
        identifier.hash(&mut hasher);
        let id = hasher.finish();

        let (reservoir, itr) = if let Some(reservoir) = self.samples.get_mut(&id) {
            (reservoir, self.itr.get_mut(&id).unwrap())
        } else {
            let new_reservoir = vec![None; self.items_per_bucket];
            let iter = self.items_per_bucket;
            self.samples.insert(id, new_reservoir);
            self.itr.insert(id, iter);
            (
                self.samples.get_mut(&id).unwrap(),
                self.itr.get_mut(&id).unwrap(),
            )
        };

        let mut rng = rand::thread_rng();
        for sample in new_bytes.chunks(self.sample_size) {
            let random_number = rng.gen_range(0..*itr);
            if random_number < reservoir.len() {
                reservoir[random_number] = Some(Bytes::copy_from_slice(sample));
            }
            *itr += 1;
        }
    }
}

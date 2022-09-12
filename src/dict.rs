use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::ops::Deref;
mod reservoir;
mod stratified;

use reservoir::ReservoirDictionaryBuilder;
use stratified::StratifiedReservoirDictionaryBuilder;

#[derive(Clone, Serialize, Deserialize)]
pub struct Dictionary(Bytes);

impl Dictionary {
    pub fn reservoir_builder(
        dict_mib: usize,
        sample_size: usize,
        reservoir_mib: usize,
    ) -> ReservoirDictionaryBuilder {
        ReservoirDictionaryBuilder::empty(dict_mib, sample_size, reservoir_mib)
    }

    pub fn stratified_reservoir_builder(
        dict_mib: usize,
        sample_size: usize,
        items_per_bucket: usize,
    ) -> StratifiedReservoirDictionaryBuilder {
        StratifiedReservoirDictionaryBuilder::empty(dict_mib, sample_size, items_per_bucket)
    }

    pub fn from(mut bytes: impl bytes::Buf) -> Self {
        Self(bytes.copy_to_bytes(bytes.remaining()))
    }
}

impl Deref for Dictionary {
    type Target = Bytes;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

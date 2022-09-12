use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::ops::Deref;
mod reservoir;
mod stratified;

use reservoir::ReservoirDictionaryBuilder;
use stratified::StratifiedReservoirDictionaryBuilder;

/// Dictionary used for RLZ compression
#[derive(Clone, Serialize, Deserialize)]
pub struct Dictionary(Bytes);

impl Dictionary {
    /// reservoir sample based dictionary builder
    #[must_use]
    pub fn reservoir_builder(
        dict_mib: usize,
        sample_size: usize,
        reservoir_mib: usize,
    ) -> ReservoirDictionaryBuilder {
        ReservoirDictionaryBuilder::empty(dict_mib, sample_size, reservoir_mib)
    }

    /// stratified reservoir sample based dictionary builder
    #[must_use]
    pub fn stratified_reservoir_builder(
        dict_mib: usize,
        sample_size: usize,
        items_per_bucket: usize,
    ) -> StratifiedReservoirDictionaryBuilder {
        StratifiedReservoirDictionaryBuilder::empty(dict_mib, sample_size, items_per_bucket)
    }

    /// Construct dictionary from existing bytes
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

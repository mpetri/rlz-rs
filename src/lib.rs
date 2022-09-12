//! Relative Lempel-Ziv compression against a fixed dictionary

#![warn(clippy::pedantic)]
#![warn(missing_docs)]

mod coder;
mod config;
mod decoder;
mod dict;
mod encoder;
mod error;
mod factor;
mod index;
mod scratch;
mod vbyte;

use bytes::{Buf, BufMut};

pub use config::Compression;
use decoder::Decoder;
pub use dict::Dictionary;
use encoder::Encoder;

pub use error::Error;

/// Main RLZ compressor class
pub struct RlzCompressor {
    dict: Dictionary,
    config: config::Compression,
    encoder: Option<Encoder>,
    decoder: Decoder,
}

impl RlzCompressor {
    /// Builder used to construct the RLZ compressor
    #[must_use]
    pub fn builder() -> RlzBuilder {
        RlzBuilder::default()
    }

    /// Encode a vector of bytes against the dictionary
    #[tracing::instrument(skip_all)]
    pub fn encode(&self, input: impl Buf, output: impl BufMut) -> Result<usize, Error> {
        if let Some(encoder) = &self.encoder {
            encoder.encode(&self.dict, input, output)
        } else {
            Err(Error::NoEncoderAvailable)
        }
    }

    /// If RlzCompressor is loaded from disk we rebuild the index to enable encoding
    pub fn enable_encode(&mut self) {
        if self.encoder.is_none() {
            tracing::info!("no encoder present. rebuilding...");
            let encoder = Encoder::build(&self.dict, &self.config);
            self.encoder = Some(encoder);
        }
    }

    /// Decode a vector of bytes that was compressed against the dictionary
    #[tracing::instrument(skip_all)]
    pub fn decode(&self, input: &[u8], output: impl std::io::Write) -> Result<usize, Error> {
        self.decoder.decode(&self.dict, input, output)
    }

    /// Store the compressor (the dict + config) on disk
    #[tracing::instrument(skip_all)]
    pub fn store(&self, output: impl std::io::Write) -> Result<(), Error> {
        let mut zstd_encoder = zstd::stream::write::Encoder::new(output, 6)?;
        bincode::serialize_into(&mut zstd_encoder, &self.dict)?;
        bincode::serialize_into(&mut zstd_encoder, &self.config)?;
        bincode::serialize_into(&mut zstd_encoder, &self.decoder)?;
        zstd_encoder.do_finish()?;
        Ok(())
    }

    /// Load the compressor (dict + config) without the index for encoding from disk
    #[tracing::instrument(skip_all)]
    pub fn load(input: impl std::io::Read) -> Result<Self, Error> {
        let mut zstd_decoder = zstd::stream::read::Decoder::new(input)?;
        let dict: Dictionary = bincode::deserialize_from(&mut zstd_decoder)?;
        let config: config::Compression = bincode::deserialize_from(&mut zstd_decoder)?;
        let decoder: Decoder = bincode::deserialize_from(&mut zstd_decoder)?;
        Ok(Self {
            dict,
            config,
            decoder,
            encoder: None,
        })
    }

    /// Load the compressor (dict + config) and rebuild the index for encoding
    #[tracing::instrument(skip_all)]
    pub fn load_and_build_encoder(&self, input: impl std::io::Read) -> Result<Self, Error> {
        let mut new_compressor = Self::load(input)?;
        new_compressor.enable_encode();
        Ok(new_compressor)
    }
}

/// RLZ compressor builder
#[derive(Default)]
pub struct RlzBuilder {
    config: config::Compression,
}

impl RlzBuilder {
    /// Specificy the minimum length of a factor
    #[must_use]
    pub fn literal_threshold(mut self, threshold: u32) -> RlzBuilder {
        self.config.literal_threshold = threshold;
        self
    }

    /// Sepcify the compression codec used for compressing factors
    #[must_use]
    pub fn factor_coder(mut self, factor_coder: coder::Coder) -> RlzBuilder {
        self.config.factor_compression = factor_coder;
        self
    }

    /// build RLZ compressor from config and dictionary
    pub fn build_from_dict(self, dict: Dictionary) -> RlzCompressor {
        let encoder = Encoder::build(&dict, &self.config);
        let decoder = Decoder::from_config(&self.config);
        RlzCompressor {
            config: self.config,
            encoder: Some(encoder),
            decoder,
            dict,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn encode_and_decode(dict: Vec<u8>,text: Vec<u8>) {
            let dict = Dictionary::from(&dict[..]);

            let rlz_compressor = RlzCompressor::builder().build_from_dict(dict);

            let mut output = Vec::new();

            let encoded_len = rlz_compressor.encode(&text[..],&mut output)?;
            assert_eq!(encoded_len,output.len());

            let mut recovered = Vec::new();
            rlz_compressor.decode(&output[..],&mut recovered)?;

            assert_eq!(recovered,text);
        }
    }

    proptest! {
        #[test]
        fn encode_store_and_decode(dict: Vec<u8>,text: Vec<u8>) {
            let dict = Dictionary::from(&dict[..]);

            let rlz_compressor = RlzCompressor::builder().build_from_dict(dict);

            let mut output = Vec::new();

            let encoded_len = rlz_compressor.encode(&text[..],&mut output)?;
            assert_eq!(encoded_len,output.len());

            let mut stored_decoder = Vec::new();
            rlz_compressor.store(&mut stored_decoder)?;

            let loaded_decoder = RlzCompressor::load(&stored_decoder[..])?;

            let mut recovered = Vec::new();
            loaded_decoder.decode(&output[..],&mut recovered)?;

            assert_eq!(recovered,text);
        }
    }
}

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
pub use decoder::Decoder;
pub use dict::Dictionary;
pub use encoder::Encoder;

pub use error::RlzError;

pub struct RlzCompressor {
    dict: Dictionary,
    config: config::Compression,
    encoder: Option<Encoder>,
    decoder: Decoder,
}

impl RlzCompressor {
    pub fn builder() -> RlzBuilder {
        RlzBuilder::default()
    }

    #[tracing::instrument(skip_all)]
    pub fn encode(&self, input: impl Buf, output: impl BufMut) -> Result<usize, RlzError> {
        if let Some(encoder) = &self.encoder {
            encoder.encode(&self.dict, input, output)
        } else {
            Err(RlzError::NoEncoderAvailable)
        }
    }

    pub fn enable_encode(&mut self) {
        if self.encoder.is_none() {
            tracing::info!("no encoder present. rebuilding...");
            let encoder = Encoder::build(&self.dict, &self.config);
            self.encoder = Some(encoder);
        }
    }

    #[tracing::instrument(skip_all)]
    pub fn decode(&self, input: &[u8], output: impl std::io::Write) -> Result<usize, RlzError> {
        self.decoder.decode(&self.dict, input, output)
    }

    #[tracing::instrument(skip_all)]
    pub fn store(&self, output: impl std::io::Write) -> Result<(), RlzError> {
        let mut zstd_encoder = zstd::stream::write::Encoder::new(output, 6)?;
        bincode::serialize_into(&mut zstd_encoder, &self.dict)?;
        bincode::serialize_into(&mut zstd_encoder, &self.config)?;
        bincode::serialize_into(&mut zstd_encoder, &self.decoder)?;
        zstd_encoder.do_finish()?;
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    pub fn load(input: impl std::io::Read) -> Result<Self, RlzError> {
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

    #[tracing::instrument(skip_all)]
    pub fn load_and_build_encoder(&self, input: impl std::io::Read) -> Result<Self, RlzError> {
        let mut new_compressor = Self::load(input)?;
        new_compressor.enable_encode();
        Ok(new_compressor)
    }
}

#[derive(Default)]
pub struct RlzBuilder {
    config: config::Compression,
}

impl RlzBuilder {
    pub fn literal_threshold(mut self, threshold: u32) -> RlzBuilder {
        self.config.literal_threshold = threshold;
        self
    }

    pub fn factor_coder(mut self, factor_coder: coder::Coder) -> RlzBuilder {
        self.config.factor_compression = factor_coder;
        self
    }

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
}

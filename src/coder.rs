use bytes::{Buf, BufMut};
use serde::{Deserialize, Serialize};

use crate::{factor::FactorType, scratch::Scratch, RlzError};

#[derive(Copy, Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct ZstdCompressor {
    level: i32,
}

impl ZstdCompressor {
    pub(crate) fn new(level: i32) -> Self {
        Self { level }
    }
}

impl ZstdCompressor {
    pub fn compress(&self, output: &mut [u8], input: &[u8]) -> Result<usize, RlzError> {
        let num_compressed_bytes = if !input.is_empty() {
            zstd::bulk::compress_to_buffer(input, output, self.level)
                .map_err(|e| RlzError::EncodingError { source: e })?
        } else {
            0
        };
        Ok(num_compressed_bytes)
    }

    pub fn decompress(&self, input: &[u8], output: &mut [u8]) -> Result<usize, RlzError> {
        if input.has_remaining() {
            let num_decompressed_bytes = zstd::bulk::decompress_to_buffer(input, output)
                .map_err(|e| RlzError::DecodingError { source: e })?;
            return Ok(num_decompressed_bytes);
        }
        Ok(0)
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct Coder {
    compressor: ZstdCompressor,
}

impl Coder {
    pub fn zstd(lvl: i32) -> Coder {
        Coder {
            compressor: ZstdCompressor::new(lvl),
        }
    }
}

impl Default for Coder {
    fn default() -> Coder {
        Coder {
            compressor: ZstdCompressor::new(6),
        }
    }
}

impl Coder {
    #[tracing::instrument(skip_all)]
    pub(crate) fn encode(
        &self,
        mut output: impl BufMut,
        scratch: &mut Scratch,
    ) -> Result<usize, RlzError> {
        // (1) ensure we have enough space
        let max_expected = scratch.literals.len() + scratch.offsets.len() + scratch.lens.len();
        scratch.reserve_encoded(max_expected);

        // (2) encode everything
        let mut written_bytes = self
            .compressor
            .compress(&mut scratch.encoded, &scratch.literals)?;
        let literal_bytes = written_bytes;
        let offset_bytes = self
            .compressor
            .compress(&mut scratch.encoded[written_bytes..], &scratch.offsets)?;
        written_bytes += offset_bytes;
        written_bytes += self
            .compressor
            .compress(&mut scratch.encoded[written_bytes..], &scratch.lens)?;

        let mut encode_bytes = written_bytes;
        encode_bytes += crate::vbyte::encode(&mut output, literal_bytes as u32);
        encode_bytes += crate::vbyte::encode(&mut output, offset_bytes as u32);

        output.put_slice(&scratch.encoded[..written_bytes]);

        Ok(encode_bytes)
    }

    #[tracing::instrument(skip_all)]
    pub(crate) fn decode(&self, mut input: &[u8], scratch: &mut Scratch) -> Result<(), RlzError> {
        let num_literal_bytes = crate::vbyte::decode(&mut input) as usize;
        let num_offset_bytes = crate::vbyte::decode(&mut input) as usize;

        // (1) ensure we have enough space
        scratch.reserve_output(input.remaining());

        let (literal_bytes, remainder) = input.split_at(num_literal_bytes as usize);
        let (offset_bytes, len_bytes) = remainder.split_at(num_offset_bytes as usize);

        // (2) perform the decoding
        let decoded = self
            .compressor
            .decompress(literal_bytes, &mut scratch.literals)?;
        scratch.literals.truncate(decoded);

        let decoded = self
            .compressor
            .decompress(offset_bytes, &mut scratch.offsets)?;
        scratch.offsets.truncate(decoded);

        let decoded = self.compressor.decompress(len_bytes, &mut scratch.lens)?;
        scratch.lens.truncate(decoded);

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    pub(crate) fn store_factor(&self, scratch: &mut Scratch, factor: FactorType) {
        match factor {
            FactorType::Literal(literal) => {
                scratch.lens.put_u32(literal.len() as u32);
                scratch.literals.put_slice(&literal);
            }
            FactorType::Copy { offset, len } => {
                scratch.offsets.put_u32(offset);
                scratch.lens.put_u32(len);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn correct_output_len(literals: Vec<u8>,offsets: Vec<u8>,lens: Vec<u8>) {
            let mut scratch =  Scratch {
                encoded: BytesMut::zeroed(1024 * 1024),
                literals: BytesMut::from(&literals[..]),
                offsets: BytesMut::from(&offsets[..]),
                lens: BytesMut::from(&lens[..]),
            };
            let mut output = Vec::new();
            let coder = Coder::default();
            let encoded_len = coder.encode(&mut output, &mut scratch)?;
            assert_eq!(encoded_len,output.len());
        }
    }

    proptest! {
        #[test]
        fn recover(literals: Vec<u8>,offsets: Vec<u8>,lens: Vec<u8>) {
            let mut scratch =  Scratch {
                encoded: BytesMut::with_capacity(1024 * 1024),
                literals: BytesMut::from(&literals[..]),
                offsets: BytesMut::from(&offsets[..]),
                lens: BytesMut::from(&lens[..]),
            };
            let mut output = Vec::new();
            let coder = Coder::zstd(3);
            let encoded_len = coder.encode(&mut output, &mut scratch)?;
            assert_eq!(encoded_len,output.len());
            dbg!(encoded_len);

            let mut scratch2 =  Scratch {
                encoded: BytesMut::with_capacity(1024 * 1024),
                literals: BytesMut::with_capacity(1024 * 1024),
                offsets: BytesMut::with_capacity(1024 * 1024),
                lens: BytesMut::with_capacity(1024 * 1024),
            };

            coder.decode(&output,&mut scratch2)?;

            assert_eq!(scratch.literals,scratch2.literals);
            assert_eq!(scratch.offsets,scratch2.offsets);
            assert_eq!(scratch.lens,scratch2.lens);
        }
    }
}

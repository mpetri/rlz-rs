use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::{
    io::{Read, Write},
    mem,
};

use crate::{scratch::Scratch, RlzError};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Compressor {
    Zlib(u32),
    Zstd(i32),
    Plain,
}

impl Compressor {
    pub fn compress(&self, mut output: &mut Vec<u8>, input: &[u8]) -> Result<usize, RlzError> {
        let len_before = output.len();
        if !input.is_empty() {
            match self {
                Compressor::Zlib(lvl) => {
                    let mut e = flate2::write::ZlibEncoder::new(
                        &mut output,
                        flate2::Compression::new(*lvl),
                    );
                    e.write_all(input)?;
                    e.finish()?;
                }
                Compressor::Zstd(lvl) => zstd::stream::copy_encode(input, &mut output, *lvl)?,
                Compressor::Plain => {
                    output.write_all(input)?;
                }
            }
        }
        Ok(output.len() - len_before)
    }

    pub fn decompress(
        &self,
        mut input: impl std::io::Read,
        output: &mut Vec<u8>,
    ) -> Result<(), RlzError> {
        match self {
            Compressor::Zlib(_lvl) => {
                let mut e = flate2::read::ZlibDecoder::new(input);
                e.read_to_end(output)?;
            }
            Compressor::Zstd(_lvl) => zstd::stream::copy_decode(input, output)?,
            Compressor::Plain => {
                input.read_to_end(output)?;
            }
        }
        Ok(())
    }

    pub fn decompress_u32(
        &self,
        mut input: impl std::io::Read,
        output: &mut Vec<u32>,
    ) -> Result<(), RlzError> {
        let mut tmp = vec![0; out_len as usize * 4];
        match self {
            Compressor::Zlib(_lvl) => {
                let mut e = flate2::read::ZlibDecoder::new(input);
                e.read_to_end(&mut tmp)?;
            }
            Compressor::Zstd(_lvl) => zstd::stream::copy_decode(input, &mut tmp)?,
            Compressor::Plain => {
                input.read_exact(&mut tmp)?;
            }
        }
        dbg!(tmp.len());

        let mut tt = &tmp[..];
        for _ in 0..out_len {
            output.push(tt.read_u32::<byteorder::LittleEndian>()?);
        }
        Ok(())
    }

    pub fn compress_u32(&self, mut output: &mut Vec<u8>, input: &[u32]) -> Result<usize, RlzError> {
        let len_before = output.len();
        dbg!(input.len());

        crate::vbyte::encode(&mut output, input.len() as u32)?;
        let input_u8: &[u8] = bytemuck::cast_slice(input);
        if !input.is_empty() {
            match self {
                Compressor::Zlib(lvl) => {
                    let mut e = flate2::write::ZlibEncoder::new(
                        &mut output,
                        flate2::Compression::new(*lvl),
                    );
                    e.write_all(input_u8)?;
                    e.finish()?;
                }
                Compressor::Zstd(lvl) => zstd::stream::copy_encode(input_u8, &mut output, *lvl)?,
                Compressor::Plain => {
                    output.write_all(input_u8)?;
                }
            }
        }
        Ok(output.len() - len_before)
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Coder {
    literal_compressor: Compressor,
    offset_compressor: Compressor,
    len_compressor: Compressor,
}

impl Coder {
    pub fn zlib(lvl: u32) -> Coder {
        Coder {
            literal_compressor: Compressor::Zlib(lvl),
            offset_compressor: Compressor::Zlib(lvl),
            len_compressor: Compressor::Zlib(lvl),
        }
    }

    pub fn zstd(lvl: i32) -> Coder {
        Coder {
            literal_compressor: Compressor::Zstd(lvl),
            offset_compressor: Compressor::Zstd(lvl),
            len_compressor: Compressor::Zstd(lvl),
        }
    }

    pub fn plain() -> Coder {
        Coder {
            literal_compressor: Compressor::Plain,
            offset_compressor: Compressor::Plain,
            len_compressor: Compressor::Plain,
        }
    }
}

impl Default for Coder {
    fn default() -> Coder {
        Coder {
            literal_compressor: Compressor::Zstd(6),
            offset_compressor: Compressor::Zstd(6),
            len_compressor: Compressor::Zstd(6),
        }
    }
}

impl Coder {
    pub(crate) fn encode(
        &self,
        mut output: impl std::io::Write,
        scratch: &mut Scratch,
    ) -> Result<usize, RlzError> {
        let literal_bytes = self
            .literal_compressor
            .compress(&mut scratch.encoded, &scratch.literals)?;
        let offset_bytes = self
            .offset_compressor
            .compress_u32(&mut scratch.encoded, &scratch.offsets)?;
        let lens_bytes = self
            .len_compressor
            .compress_u32(&mut scratch.encoded, &scratch.lens)?;

        crate::vbyte::encode(&mut output, literal_bytes as u32)?;
        crate::vbyte::encode(&mut output, offset_bytes as u32)?;
        crate::vbyte::encode(&mut output, lens_bytes as u32)?;

        std::io::copy(&mut &scratch.encoded[..], &mut output)?;

        Ok(literal_bytes + offset_bytes + lens_bytes)
    }

    pub(crate) fn decode(
        &self,
        mut input: impl std::io::Read,
        scratch: &mut Scratch,
    ) -> Result<(), RlzError> {
        let literal_bytes = crate::vbyte::decode(&mut input)?;
        let offset_bytes = crate::vbyte::decode(&mut input)?;
        let lens_bytes = crate::vbyte::decode(&mut input)?;

        self.literal_compressor
            .decompress(&mut input, literal_bytes, &mut scratch.literals)?;
        self.offset_compressor
            .decompress_u32(&mut input, &mut scratch.offsets)?;
        self.len_compressor
            .decompress_u32(&mut input, &mut scratch.lens)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn correct_output_len(literals: Vec<u8>,offsets: Vec<u32>,lens: Vec<u32>) {
            let mut scratch =  Scratch {
                encoded: Vec::with_capacity(1234),
                literals,
                offsets,
                lens,
            };
            let mut output = Vec::new();
            let coder = Coder::default();
            let encoded_len = coder.encode(&mut output, &mut scratch)?;
            assert_eq!(encoded_len,output.len());
        }
    }

    #[test]
    fn debug() {
        let mut scratch = Scratch {
            encoded: Vec::with_capacity(1234),
            literals: Vec::new(),
            offsets: vec![0],
            lens: Vec::new(),
        };
        let mut output = Vec::new();
        let coder = Coder::plain();
        let encoded_len = coder.encode(&mut output, &mut scratch).unwrap();
        assert_eq!(encoded_len, output.len());
        dbg!(encoded_len);

        let mut scratch2 = Scratch {
            encoded: Vec::with_capacity(1234),
            literals: Vec::with_capacity(1234),
            offsets: Vec::with_capacity(1234),
            lens: Vec::with_capacity(1234),
        };

        coder.decode(&output[..], &mut scratch2).unwrap();

        assert_eq!(scratch.literals, scratch2.literals);
        assert_eq!(scratch.offsets, scratch2.offsets);
        assert_eq!(scratch.lens, scratch2.lens);
    }

    proptest! {
        #[test]
        fn recover(literals: Vec<u8>,offsets: Vec<u32>,lens: Vec<u32>) {
            let mut scratch =  Scratch {
                encoded: Vec::with_capacity(1234),
                literals,
                offsets,
                lens,
            };
            let mut output = Vec::new();
            let coder = Coder::zstd(3);
            let encoded_len = coder.encode(&mut output, &mut scratch)?;
            assert_eq!(encoded_len,output.len());
            dbg!(encoded_len);

            let mut scratch2 =  Scratch {
                encoded: Vec::with_capacity(1234),
                literals: Vec::with_capacity(1234),
                offsets: Vec::with_capacity(1234),
                lens: Vec::with_capacity(1234),
            };

            coder.decode(&output[..],&mut scratch2)?;

            assert_eq!(scratch.literals,scratch2.literals);
            assert_eq!(scratch.offsets,scratch2.offsets);
            assert_eq!(scratch.lens,scratch2.lens);
        }
    }

    proptest! {
        #[test]
        fn recover_zlib(literals: Vec<u8>,offsets: Vec<u32>,lens: Vec<u32>) {
            let mut scratch =  Scratch {
                encoded: Vec::with_capacity(1234),
                literals,
                offsets,
                lens,
            };
            let mut output = Vec::new();
            let coder = Coder::zlib(3);
            let encoded_len = coder.encode(&mut output, &mut scratch)?;
            assert_eq!(encoded_len,output.len());

            let mut scratch2 =  Scratch {
                encoded: Vec::with_capacity(1234),
                literals: Vec::with_capacity(1234),
                offsets: Vec::with_capacity(1234),
                lens: Vec::with_capacity(1234),
            };

            coder.decode(&output[..],&mut scratch2)?;

            assert_eq!(scratch.literals,scratch2.literals);
            assert_eq!(scratch.offsets,scratch2.offsets);
            assert_eq!(scratch.lens,scratch2.lens);
        }
    }
}

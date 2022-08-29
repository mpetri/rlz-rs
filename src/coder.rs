use crate::{scratch::Scratch, RlzError};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Compressor {
    Zlib(u32),
    Zstd(u32),
    Plain,
    Vbyte,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Coder {
    literal_compressor: Compressor,
    offset_compressor: Compressor,
    len_compressor: Compressor,
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
        output: impl std::io::Write,
        scratch: &Scratch,
    ) -> Result<usize, RlzError> {
        Ok(0)
    }
}

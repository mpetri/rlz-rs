use crate::factor::FactorType;
use crate::{scratch::Scratch, RlzError};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Compressor {
    Zlib(u32),
    Zstd(u32),
    Plain,
    Vbyte,
}

impl Compressor {
    pub fn compress(&self, output: impl std::io::Write, input: &[u8]) -> Result<usize, RlzError> {
        Ok(0)
    }

    pub fn compress_u32(
        &self,
        output: impl std::io::Write,
        input: &[u32],
    ) -> Result<usize, RlzError> {
        let input_u8: &[u8] = bytemuck::cast_slice(input);
        self.compress(output, input_u8)
    }
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
        mut output: impl std::io::Write,
        scratch: &Scratch,
    ) -> Result<usize, RlzError> {
        let literal_size = self
            .literal_compressor
            .compress(&mut output, &scratch.literals)?;
        let offset_size = self
            .offset_compressor
            .compress_u32(&mut output, &scratch.offsets)?;
        let lens_size = self
            .len_compressor
            .compress_u32(&mut output, &scratch.lens)?;

        Ok(literal_size + offset_size + lens_size)
    }

    pub(crate) fn store_factor(&self, factor: FactorType, scratch: &mut Scratch) {
        match factor {
            FactorType::Literal(literal) => {
                scratch.lens.push(literal.len() as u32);
                scratch.literals.copy_from_slice(literal);
            }
            FactorType::Copy { offset, len } => {
                scratch.offsets.push(offset);
                scratch.lens.push(len);
            }
        }
    }
}

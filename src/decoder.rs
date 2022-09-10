use bytes::Buf;

use crate::{coder, config, dict::Dictionary, factor::FactorType, scratch, RlzError};

pub struct Decoder<'index> {
    dict: &'index Dictionary,
    scratch: scratch::ScratchSpace,
    config: config::Compression,
    coder: coder::Coder,
}

impl<'index> Decoder<'index> {
    pub fn decode(
        &self,
        input: bytes::Bytes,
        mut output: impl std::io::Write,
    ) -> Result<usize, RlzError> {
        let mut scratch = self.scratch.get();
        scratch.clear();

        self.coder.decode(input, &mut scratch)?;

        for factor in EncodedFactorIterator::new(&mut scratch, &self.config) {
            match factor {
                FactorType::Literal(literal) => {
                    output.write_all(&literal)?;
                }
                FactorType::Copy { offset, len } => {
                    let offset = offset as usize;
                    let dict_slice = &self.dict[offset..offset + len as usize];
                    output.write_all(dict_slice)?;
                }
            }
        }

        Ok(0)
    }
}

struct EncodedFactorIterator<'scratch, 'decoder> {
    scratch: &'scratch mut scratch::Scratch,
    config: &'decoder config::Compression,
}

impl<'scratch, 'decoder> EncodedFactorIterator<'scratch, 'decoder> {
    fn new(scratch: &'scratch mut scratch::Scratch, config: &'decoder config::Compression) -> Self {
        Self { scratch, config }
    }
}

impl<'scratch, 'decoder> Iterator for EncodedFactorIterator<'scratch, 'decoder> {
    type Item = FactorType;

    fn next(&mut self) -> Option<Self::Item> {
        let remaining = self.scratch.lens.has_remaining();
        if remaining {
            let len = self.scratch.lens.get_u32();
            if len <= self.config.literal_threshold {
                let literal_slice = self.scratch.literals.copy_to_bytes(len as usize);
                Some(FactorType::Literal(literal_slice))
            } else {
                let offset = self.scratch.offsets.get_u32();
                Some(FactorType::Copy { offset, len })
            }
        } else {
            None
        }
    }
}

use crate::{coder, config, dict::Dictionary, factor::FactorType, scratch, RlzError};

pub struct Decoder<'index> {
    dict: &'index Dictionary,
    scratch: scratch::ScratchSpace,
    config: config::Compression,
    coder: coder::Coder,
}

impl<'index> Decoder<'index> {
    pub fn decode(&self, input: &[u8], mut output: impl std::io::Write) -> Result<usize, RlzError> {
        let mut scratch = self.scratch.get();
        scratch.clear();

        self.coder.decode(input, &mut scratch);

        for factor in EncodedFactorIterator::new(&scratch, &self.config) {
            match factor {
                FactorType::Literal(literal) => {
                    output.write_all(literal)?;
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
    cur: usize,
    literal_offset: usize,
    copy_offset: usize,
    scratch: &'scratch scratch::Scratch,
    config: &'decoder config::Compression,
}

impl<'scratch, 'decoder> EncodedFactorIterator<'scratch, 'decoder> {
    fn new(scratch: &'scratch scratch::Scratch, config: &'decoder config::Compression) -> Self {
        Self {
            cur: 0,
            literal_offset: 0,
            copy_offset: 0,
            scratch,
            config,
        }
    }
}

impl<'scratch, 'decoder> Iterator for EncodedFactorIterator<'scratch, 'decoder> {
    type Item = FactorType<'scratch>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(&len) = self.scratch.lens.get(self.cur) {
            self.cur += 1;
            if len <= self.config.literal_threshold {
                let literal_slice =
                    &self.scratch.literals[self.literal_offset..self.literal_offset + len as usize];
                self.literal_offset += len as usize;
                Some(FactorType::Literal(literal_slice))
            } else {
                let offset = self.scratch.offsets[self.copy_offset];
                self.copy_offset += 1;
                Some(FactorType::Copy { offset, len })
            }
        } else {
            None
        }
    }
}

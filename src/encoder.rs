use crate::{
    coder, config, dict,
    factor::{self, FactorType},
    index, scratch, RlzError,
};

pub struct Encoder {
    index: index::Index,
    coder: coder::Coder,
    scratch: scratch::ScratchSpace,
}

impl Encoder {
    pub fn builder() -> EncoderBuilder {
        EncoderBuilder::default()
    }

    pub fn encode(&self, input: &[u8], output: impl std::io::Write) -> Result<usize, RlzError> {
        let mut scratch = self.scratch.get();
        for factor in self.index.factorize(input) {
            match factor {
                FactorType::Literal(factor_literals) => {
                    scratch.literals.extend_from_slice(factor_literals)
                }
                FactorType::Copy { offset, len } => {
                    scratch.offsets.push(offset);
                    scratch.lens.push(len);
                }
            }
        }
        let encode_output = self.coder.encode(output, &scratch);
        self.scratch.release(scratch);
        encode_output
    }
}

#[derive(Default)]
pub struct EncoderBuilder {
    compression_config: config::Compression,
}

impl EncoderBuilder {
    pub fn local_search(mut self, window_bytes: usize) -> EncoderBuilder {
        self.compression_config.local_search = factor::LocalSearch::Window(window_bytes);
        self
    }

    pub fn literal_threshold(mut self, threshold: usize) -> EncoderBuilder {
        self.compression_config.literal_threshold = threshold;
        self
    }

    pub fn factor_coder(mut self, factor_coder: coder::Coder) -> EncoderBuilder {
        self.compression_config.factor_compression = factor_coder;
        self
    }

    pub fn build(self, dict: dict::Dictionary) -> Encoder {
        let index = index::Index::from_dict(dict, &self.compression_config);
        Encoder {
            index,
            coder: self.compression_config.factor_compression,
            scratch: scratch::ScratchSpace::default(),
        }
    }
}

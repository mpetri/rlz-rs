use bytes::{Buf, BufMut};

use crate::{
    coder, config, dict,
    factor::{self},
    index, scratch, RlzError,
};

pub struct Encoder {
    pub(crate) index: index::Index,
    pub(crate) coder: coder::Coder,
    scratch: scratch::ScratchSpace,
}

impl Encoder {
    pub fn builder() -> EncoderBuilder {
        EncoderBuilder::default()
    }

    #[tracing::instrument(skip_all)]
    pub fn encode(&self, input: impl Buf, output: impl BufMut) -> Result<usize, RlzError> {
        let mut scratch = self.scratch.get();
        scratch.clear();
        for factor in self.index.factorize(input) {
            self.coder.store_factor(&mut scratch, factor);
        }
        let encode_output = self.coder.encode(output, &mut scratch);
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

    pub fn literal_threshold(mut self, threshold: u32) -> EncoderBuilder {
        self.compression_config.literal_threshold = threshold;
        self
    }

    pub fn factor_coder(mut self, factor_coder: coder::Coder) -> EncoderBuilder {
        self.compression_config.factor_compression = factor_coder;
        self
    }

    #[tracing::instrument(skip_all)]
    pub fn build(self, dict: dict::Dictionary) -> Encoder {
        let index = index::Index::from_dict(dict, &self.compression_config);
        Encoder {
            index,
            coder: self.compression_config.factor_compression,
            scratch: scratch::ScratchSpace::default(),
        }
    }
}

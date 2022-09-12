use bytes::{Buf, BufMut};

use crate::{coder, config, dict, index, scratch, Error};

pub(crate) struct Encoder {
    pub(crate) index: index::Index,
    pub(crate) coder: coder::Coder,
    scratch: scratch::ScratchSpace,
}

impl Encoder {
    #[tracing::instrument(skip_all)]
    pub(crate) fn build(
        dict: &dict::Dictionary,
        compression_config: &config::Compression,
    ) -> Encoder {
        let index = index::Index::from_dict(dict, compression_config);
        Encoder {
            index,
            coder: compression_config.factor_compression.clone(),
            scratch: scratch::ScratchSpace::default(),
        }
    }

    #[tracing::instrument(skip_all)]
    pub(crate) fn encode(
        &self,
        dict: &dict::Dictionary,
        input: impl Buf,
        output: impl BufMut,
    ) -> Result<usize, Error> {
        let mut scratch = self.scratch.get();
        scratch.clear();
        for factor in self.index.factorize(dict, input) {
            self.coder.store_factor(&mut scratch, factor);
        }
        let encode_output = self.coder.encode(output, &mut scratch);
        self.scratch.release(scratch);
        encode_output
    }
}

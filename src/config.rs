use crate::coder;
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct Compression {
    pub literal_threshold: u32,
    pub factor_compression: coder::Coder,
}

impl Compression {
    fn new() -> Compression {
        Compression {
            literal_threshold: 3,
            factor_compression: coder::Coder::default(),
        }
    }
}

impl Default for Compression {
    fn default() -> Compression {
        Compression::new()
    }
}

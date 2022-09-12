use crate::coder;
use serde::{Deserialize, Serialize};

/// Compression configuration
#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct Compression {
    /// Minimum lens before something is coded relative to the dict
    pub literal_threshold: u32,
    /// Compression codec for factors, literals
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

use crate::coder;
use serde::{Deserialize, Serialize};

/// Compression configuration
#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct Configuration {
    /// Minimum lens before something is coded relative to the dict
    pub literal_threshold: u32,
    /// Compression codec for factors, literals
    pub factor_compression: coder::Coder,
}

impl Configuration {
    fn new() -> Configuration {
        Configuration {
            literal_threshold: 3,
            factor_compression: coder::Coder::default(),
        }
    }
}

impl Default for Configuration {
    fn default() -> Configuration {
        Configuration::new()
    }
}

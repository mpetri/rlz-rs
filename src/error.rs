use thiserror::Error;

/// RLZ Error type
#[derive(Error, Debug)]
pub enum Error {
    /// Zstd encoding error
    #[error("Encoding factors error")]
    EncodingError {
        /// error source
        source: std::io::Error,
    },
    /// zstd decoding error
    #[error("Decoding factors Error")]
    DecodingError {
        /// error source
        source: std::io::Error,
    },
    /// I/O error
    #[error("I/O Error")]
    IOError(#[from] std::io::Error),
    /// unknown error
    #[error("Unknown rlz error")]
    Unknown,
    /// encoder is not available for encoding stuff
    #[error("No encoder available. Build it with enable_encode()")]
    NoEncoderAvailable,
    /// serialize/deserialize error of the rlz compressor
    #[error("Bincode serialization Error")]
    SerializeError(#[from] bincode::Error),
}

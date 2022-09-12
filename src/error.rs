use thiserror::Error;

#[derive(Error, Debug)]
pub enum RlzError {
    #[error("Encoding factors error")]
    EncodingError { source: std::io::Error },
    #[error("Decoding factors Error")]
    DecodingError { source: std::io::Error },
    #[error("I/O Error")]
    IOError(#[from] std::io::Error),
    #[error("Unknown rlz error")]
    Unknown,
    #[error("No encoder available. Build it with enable_encode()")]
    NoEncoderAvailable,
    #[error("Bincode serialization Error")]
    SerializeError(#[from] bincode::Error),
}

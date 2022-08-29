use thiserror::Error;

#[derive(Error, Debug)]
pub enum RlzError {
    #[error("I/O Error")]
    IOError(#[from] std::io::Error),
    #[error("unknown rlz error")]
    Unknown,
}

mod coder;
mod config;
mod decoder;
mod dict;
mod encoder;
mod error;
mod factor;
mod index;
mod scratch;

pub use config::Compression;
pub use decoder::Decoder;
pub use encoder::Encoder;
pub use encoder::EncoderBuilder;

pub use error::RlzError;

#[cfg(test)]
mod tests {
    use super::*;
}

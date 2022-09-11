mod coder;
mod config;
mod decoder;
mod dict;
mod encoder;
mod error;
mod factor;
mod index;
mod scratch;
mod vbyte;

pub use config::Compression;
pub use decoder::Decoder;
pub use dict::Dictionary;
pub use encoder::Encoder;
pub use encoder::EncoderBuilder;

pub use error::RlzError;

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn encode_and_decode(dict: Vec<u8>,text: Vec<u8>) {
            let dict = Dictionary::from(&dict[..]);
            let encoder = Encoder::builder().build(dict);
            let decoder = Decoder::from_encoder(&encoder);
            let mut output = Vec::new();

            let encoded_len = encoder.encode(&text[..],&mut output)?;
            assert_eq!(encoded_len,output.len());

            let mut recovered = Vec::new();
            decoder.decode(bytes::Bytes::copy_from_slice(&output[..]),&mut recovered)?;

            assert_eq!(recovered,text);
        }
    }
}

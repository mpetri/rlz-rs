use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

pub fn decode(mut data: impl std::io::Read) -> std::io::Result<u32> {
    let mut val: u32 = 0;
    let mut bytes = 0;
    loop {
        let c = data.read_u8()?;
        val += ((c & 127) as u32) << (bytes * 7);
        if (c & 128) != 0 {
            return Ok(val);
        }
        bytes += 1;
    }
    Ok(0)
}

#[allow(clippy::identity_op)]
pub fn encode(mut output: impl std::io::Write, num: u32) -> std::io::Result<()> {
    match num {
        0..=127 => output.write_u8(num as u8 | (1 << 7)),
        128..=16383 => {
            output.write_u8(((num) & ((1 << 7) - 1)) as u8)?;
            output.write_u8((num >> (7 * 1)) as u8 | (1 << 7))
        }
        16384..=2097151 => {
            output.write_u8(((num) & ((1 << 7) - 1)) as u8)?;
            output.write_u8(((num >> (7 * 1)) & ((1 << 7) - 1)) as u8)?;
            output.write_u8((num >> (7 * 2)) as u8 | (1 << 7))
        }
        2097152..=268435455 => {
            output.write_u8(((num) & ((1 << 7) - 1)) as u8)?;
            output.write_u8(((num >> (7 * 1)) & ((1 << 7) - 1)) as u8)?;
            output.write_u8(((num >> (7 * 2)) & ((1 << 7) - 1)) as u8)?;
            output.write_u8((num >> (7 * 3)) as u8 | (1 << 7))
        }
        268435456..=u32::MAX => {
            output.write_u8(((num) & ((1 << 7) - 1)) as u8)?;
            output.write_u8(((num >> (7 * 1)) & ((1 << 7) - 1)) as u8)?;
            output.write_u8(((num >> (7 * 2)) & ((1 << 7) - 1)) as u8)?;
            output.write_u8(((num >> (7 * 3)) & ((1 << 7) - 1)) as u8)?;
            output.write_u8((num >> (7 * 4)) as u8 | (1 << 7))
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn encode_and_decode_single(num: u32)  {
            let mut buf = Vec::with_capacity(6);
            super::encode(&mut buf, num)?;
            let decoded = super::decode(&buf[..])?;
            assert_eq!(decoded,num)
        }
    }
}

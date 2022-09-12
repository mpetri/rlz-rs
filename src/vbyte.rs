use bytes::{Buf, BufMut};

pub fn decode(mut input: impl Buf) -> u32 {
    let mut val: u32 = 0;
    let mut bytes = 0;
    loop {
        let c = input.get_u8();
        val += u32::from(c & 127) << (bytes * 7);
        if (c & 128) != 0 {
            return val;
        }
        bytes += 1;
    }
}

#[allow(clippy::identity_op, clippy::cast_possible_truncation)]
pub fn encode(mut output: impl BufMut, num: u32) -> usize {
    match num {
        0..=127 => {
            output.put_u8(num as u8 | (1 << 7));
            1
        }
        128..=16383 => {
            output.put_u8(((num) & ((1 << 7) - 1)) as u8);
            output.put_u8((num >> (7 * 1)) as u8 | (1 << 7));
            2
        }
        16384..=2_097_151 => {
            output.put_u8(((num) & ((1 << 7) - 1)) as u8);
            output.put_u8(((num >> (7 * 1)) & ((1 << 7) - 1)) as u8);
            output.put_u8((num >> (7 * 2)) as u8 | (1 << 7));
            3
        }
        2_097_152..=268_435_455 => {
            output.put_u8(((num) & ((1 << 7) - 1)) as u8);
            output.put_u8(((num >> (7 * 1)) & ((1 << 7) - 1)) as u8);
            output.put_u8(((num >> (7 * 2)) & ((1 << 7) - 1)) as u8);
            output.put_u8((num >> (7 * 3)) as u8 | (1 << 7));
            4
        }
        268_435_456..=u32::MAX => {
            output.put_u8(((num) & ((1 << 7) - 1)) as u8);
            output.put_u8(((num >> (7 * 1)) & ((1 << 7) - 1)) as u8);
            output.put_u8(((num >> (7 * 2)) & ((1 << 7) - 1)) as u8);
            output.put_u8(((num >> (7 * 3)) & ((1 << 7) - 1)) as u8);
            output.put_u8((num >> (7 * 4)) as u8 | (1 << 7));
            5
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
            super::encode(&mut buf, num);
            let decoded = super::decode(&buf[..]);
            assert_eq!(decoded,num);
        }
    }
}

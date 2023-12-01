//! Encoding and decoding of zbase32.
//!
//! This is an implementation of the human-oriented base-32 encoding called
//! [zbase32](https://philzimmermann.com/docs/human-oriented-base-32-encoding.txt).

#![cfg_attr(all(test, feature = "unstable"), feature(test))]

/// Alphabet used by zbase32
pub const ALPHABET: &[u8; 32] = b"ybndrfg8ejkmcpqxot1uwisza345h769";

#[rustfmt::skip]
const CONVERSION_TABLE: &[Option<u8>; 256] = &[
    /*   0 */ None,       None,       None,       None,       None,
    /*   5 */ None,       None,       None,       None,       None,
    /*  10 */ None,       None,       None,       None,       None,
    /*  15 */ None,       None,       None,       None,       None,
    /*  20 */ None,       None,       None,       None,       None,
    /*  25 */ None,       None,       None,       None,       None,
    /*  30 */ None,       None,       None,       None,       None,
    /*  35 */ None,       None,       None,       None,       None,
    /*  40 */ None,       None,       None,       None,       None,
    /*  45 */ None,       None,       None,       None,       Some(0x12),
    /*  50 */ None,       Some(0x19), Some(0x1a), Some(0x1b), Some(0x1e),
    /*  55 */ Some(0x1d), Some(0x07), Some(0x1f), None,       None,
    /*  60 */ None,       None,       None,       None,       None,
    /*  65 */ None,       None,       None,       None,       None,
    /*  70 */ None,       None,       None,       None,       None,
    /*  75 */ None,       None,       None,       None,       None,
    /*  80 */ None,       None,       None,       None,       None,
    /*  85 */ None,       None,       None,       None,       None,
    /*  90 */ None,       None,       None,       None,       None,
    /*  95 */ None,       None,       Some(0x18), Some(0x01), Some(0x0c),
    /* 100 */ Some(0x03), Some(0x08), Some(0x05), Some(0x06), Some(0x1c),
    /* 105 */ Some(0x15), Some(0x09), Some(0x0a), None,       Some(0x0b),
    /* 110 */ Some(0x02), Some(0x10), Some(0x0d), Some(0x0e), Some(0x04),
    /* 115 */ Some(0x16), Some(0x11), Some(0x13), None,       Some(0x14),
    /* 120 */ Some(0x0f), Some(0x00), Some(0x17), None,       None,
    /* 125 */ None,       None,       None,       None,       None,
    /* 130 */ None,       None,       None,       None,       None,
    /* 135 */ None,       None,       None,       None,       None,
    /* 140 */ None,       None,       None,       None,       None,
    /* 145 */ None,       None,       None,       None,       None,
    /* 150 */ None,       None,       None,       None,       None,
    /* 155 */ None,       None,       None,       None,       None,
    /* 160 */ None,       None,       None,       None,       None,
    /* 165 */ None,       None,       None,       None,       None,
    /* 170 */ None,       None,       None,       None,       None,
    /* 175 */ None,       None,       None,       None,       None,
    /* 180 */ None,       None,       None,       None,       None,
    /* 185 */ None,       None,       None,       None,       None,
    /* 190 */ None,       None,       None,       None,       None,
    /* 195 */ None,       None,       None,       None,       None,
    /* 200 */ None,       None,       None,       None,       None,
    /* 205 */ None,       None,       None,       None,       None,
    /* 210 */ None,       None,       None,       None,       None,
    /* 215 */ None,       None,       None,       None,       None,
    /* 220 */ None,       None,       None,       None,       None,
    /* 225 */ None,       None,       None,       None,       None,
    /* 230 */ None,       None,       None,       None,       None,
    /* 235 */ None,       None,       None,       None,       None,
    /* 240 */ None,       None,       None,       None,       None,
    /* 245 */ None,       None,       None,       None,       None,
    /* 250 */ None,       None,       None,       None,       None,
    /* 255 */ None
];

#[inline]
fn value_of_digit(digit: u8) -> Result<u8, &'static str> {
    match CONVERSION_TABLE[digit as usize] {
        Some(v) => Ok(v),
        None => Err("not a zbase32 digit"),
    }
}

/// Decode first N `bits` of given zbase32 encoded data
///
/// # Panic
///
/// Panics if `zbase32` decoded is shorter than N `bits`.
///
/// # Examples
///
/// ```
/// use zbase32;
///
/// assert_eq!(zbase32::decode(b"o", 1).unwrap(), &[0x80]);
/// ```
pub fn decode(zbase32: &[u8], bits: u64) -> Result<Vec<u8>, &'static str> {
    assert!(zbase32.len() as u64 * 5 >= bits, "zbase32 slice too short");
    #[allow(clippy::cast_possible_truncation)]
    let capacity = if bits % 8 == 0 {
        bits / 8
    } else {
        bits / 8 + 1
    } as usize;
    let mut result = Vec::with_capacity(capacity);

    let mut bits_remaining = bits;
    let mut buffer_size: u8 = 0;
    let mut buffer: u16 = !0;
    for digit in zbase32 {
        let value = value_of_digit(*digit)?;
        buffer = (buffer << 5) | u16::from(value);
        buffer_size += 5;
        if bits_remaining < 8 && u64::from(buffer_size) >= bits_remaining {
            break;
        }
        if buffer_size >= 8 {
            #[allow(clippy::cast_possible_truncation)]
            let byte = (buffer >> (buffer_size - 8)) as u8;
            result.push(byte);
            bits_remaining -= 8;
            buffer_size -= 8;
        }
    }
    if bits_remaining > 0 {
        #[allow(clippy::cast_possible_truncation)]
        let trim_right = buffer_size - bits_remaining as u8;
        buffer >>= trim_right;
        buffer_size -= trim_right;
        #[allow(clippy::cast_possible_truncation)]
        let byte = (buffer << (8_u8 - buffer_size)) as u8;
        result.push(byte);
    }
    debug_assert_eq!(capacity, result.len());
    Ok(result)
}

/// Decode given zbase32 encoded string
///
/// Just like `decode` but doesn't allow decoding with bit precision.
///
/// This decodes full bytes. For instance, if you have `b"yy"`, you'll get one
/// byte back. `b"yy"` can enode 10 bits (2 * 5) which is truncated at the next
/// lower byte boundary.
///
/// # Examples
///
/// ```
/// use zbase32;
///
/// assert_eq!(zbase32::decode_full_bytes(b"qb1ze3m1").unwrap(), b"peter");
/// ```
#[inline]
pub fn decode_full_bytes(zbase: &[u8]) -> Result<Vec<u8>, &'static str> {
    let size = zbase.len() as u64 * 5;
    decode(zbase, size / 8 * 8)
}

/// Decode first N `bits` of given zbase32 encoded string
///
/// # Panic
///
/// Panics if `zbase32` decoded is shorter than N `bits`.
///
/// # Examples
///
/// ```
/// use zbase32;
///
/// assert_eq!(zbase32::decode_str("o", 1).unwrap(), &[0x80]);
/// ```
#[inline]
pub fn decode_str(zbase32: &str, bits: u64) -> Result<Vec<u8>, &'static str> {
    decode(zbase32.as_bytes(), bits)
}

/// Decode given zbase32 encoded string
///
/// Just like `decode_str` but doesn't allow decoding with bit precision.
///
/// # Examples
///
/// ```
/// use zbase32;
///
/// assert_eq!(zbase32::decode_full_bytes_str("qb1ze3m1").unwrap(), b"peter");
/// ```
#[inline]
pub fn decode_full_bytes_str(zbase32: &str) -> Result<Vec<u8>, &'static str> {
    decode_full_bytes(zbase32.as_bytes())
}

/// Encode first N `bits` with zbase32.
///
/// # Panics
///
/// Panics if `data` is shorter than N `bits`.
///
/// # Examples
///
/// ```
/// use zbase32;
///
/// assert_eq!(zbase32::encode(b"testdata", 64), "qt1zg7drcf4gn");
/// ```
///
pub fn encode(data: &[u8], bits: u64) -> String {
    assert!(data.len() as u64 * 8 >= bits, "slice too short");
    #[allow(clippy::cast_possible_truncation)]
    let capacity = if bits % 5 == 0 {
        bits / 5
    } else {
        bits / 5 + 1
    } as usize;
    let mut result = Vec::with_capacity(capacity);

    let mut bits_remaining = bits;
    let mut bit_offset: u8 = 16;
    let mut remaining = data;
    let mut buffer = !0;

    while bits_remaining > 0 {
        if bit_offset >= 8 {
            if let Some((first, others)) = remaining.split_first() {
                buffer = buffer << 8 | u16::from(*first);
                remaining = others;
                bit_offset -= 8;
            }
        }

        let unused_bits = 5_u64.saturating_sub(bits_remaining);
        #[allow(clippy::cast_possible_truncation)]
        let index = (buffer >> (unused_bits as u8 + 16 - 5 - bit_offset) << unused_bits) & 0x1f;
        result.push(ALPHABET[index as usize]);

        bit_offset += 5;
        bits_remaining -= 5 - unused_bits;
    }

    debug_assert_eq!(capacity, result.len());
    unsafe { String::from_utf8_unchecked(result) }
}

/// Encode full bytes using zbase32.
///
/// Just like `encode` but doesn't allow encoding with bit precision.
///
/// # Examples
///
/// ```
/// use zbase32;
///
/// let data = "Just an arbitrary sentence.";
/// assert_eq!(zbase32::encode_full_bytes(data.as_bytes()),
///            "jj4zg7bycfznyam1cjwzehubqjh1yh5fp34gk5udcwzy");
/// ```
#[inline]
pub fn encode_full_bytes(data: &[u8]) -> String {
    encode(data, data.len() as u64 * 8)
}

/// Check if `data` is valid zbase32 encoded bytes
///
/// # Examples
///
/// ```
/// use zbase32;
///
/// assert!(zbase32::validate(b"y1"));
/// assert!(!zbase32::validate(b"A"));
/// ```
pub fn validate(data: &[u8]) -> bool {
    data.iter().all(|i| value_of_digit(*i).is_ok())
}

/// Check if `data` is valid zbase32 encoded string
///
/// # Examples
///
/// ```
/// use zbase32;
///
/// assert!(zbase32::validate_str("y1"));
/// assert!(!zbase32::validate_str("A"));
/// ```
#[inline(always)]
pub fn validate_str(data: &str) -> bool {
    validate(data.as_bytes())
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "unstable")]
    extern crate test;
    extern crate rand;

    use super::*;
    #[cfg(feature = "unstable")]
    use tests::rand::seq::SliceRandom;
    #[cfg(feature = "unstable")]
    use tests::rand::Rng;

    #[rustfmt::skip]
    const TEST_DATA: &[(u64, &str, &[u8])] = &[
        (0,   "",       &[]),
        (1,   "y",      &[0x00]),
        (1,   "o",      &[0x80]),
        (2,   "e",      &[0x40]),
        (2,   "a",      &[0xc0]),
        (8,   "yy",     &[0x00]),
        (10,  "yy",     &[0x00, 0x00]),
        (10,  "on",     &[0x80, 0x80]),
        (20,  "tqre",   &[0x8b, 0x88, 0x80]),
        (24,  "6n9hq",  &[0xf0, 0xbf, 0xc7]),
        (24,  "4t7ye",  &[0xd4, 0x7a, 0x04]),
        (30,  "6im5sd", &[0xf5, 0x57, 0xbb, 0x0c]),
        (160, "ybndrfg8ejkmcpqxot1uwisza345h769", &[0x00, 0x44, 0x32, 0x14, 0xc7, 0x42, 0x54, 0xb6,
                                                    0x35, 0xcf, 0x84, 0x65, 0x3a, 0x56, 0xd7, 0xc6,
                                                    0x75, 0xbe, 0x77, 0xdf])
    ];

    const INVALID_TEST_DATA: &[&str] = &["ybndrfg8ejkmcpqxot1uwisza345H769", "bnℕe", "uv", "l"];

    #[cfg(feature = "unstable")]
    const ONE_MIB: usize = 1048576;

    #[test]
    fn test_decode() {
        for &(bits, zbase32, data) in TEST_DATA {
            assert_eq!(decode(zbase32.as_bytes(), bits).unwrap(), data);
        }
    }

    #[test]
    #[rustfmt::skip]
    fn test_decode_full_bytes() {
        let test_data: &[(&str, &[u8])] = &[
            ("9",     &[]),
            ("y9",    &[0x07]),
            ("6n9hq", &[0xf0, 0xbf, 0xc7]),
            ("4t7ye", &[0xd4, 0x7a, 0x04]),
            ("ybndrfg8ejkmcpqxot1uwisza345h769", &[0x00, 0x44, 0x32, 0x14, 0xc7, 0x42, 0x54, 0xb6,
                                                   0x35, 0xcf, 0x84, 0x65, 0x3a, 0x56, 0xd7, 0xc6,
                                                   0x75, 0xbe, 0x77, 0xdf])
        ];

        for &(zbase32, data) in test_data {
            assert_eq!(decode_full_bytes_str(zbase32).unwrap(), data);
        }
    }

    #[test]
    fn test_decode_invalid_digits() {
        for string in INVALID_TEST_DATA.iter() {
            assert!(decode(string.as_bytes(), string.as_bytes().len() as u64 * 5).is_err());
            assert!(decode_full_bytes(string.as_bytes()).is_err());
        }
    }

    #[test]
    fn test_decode_superfluous_bits() {
        assert_eq!(decode(b"999", 1).unwrap(), &[0x80]);
        assert_eq!(decode(b"4t7yj", 24).unwrap(), &[0xd4, 0x7a, 0x04]);
        assert_eq!(decode(b"4t7ye9", 25).unwrap(), &[0xd4, 0x7a, 0x04, 0x00]);
        assert_eq!(decode(b"gyh3", 18).unwrap(), &[0x30, 0x39, 0x80]);
    }

    #[test]
    #[should_panic(expected = "zbase32 slice too short")]
    fn test_decode_short_slice() {
        decode(b"oyoy", 4 * 5 + 1).unwrap();
    }

    #[test]
    fn test_encode() {
        for &(bits, zbase32, data) in TEST_DATA {
            assert_eq!(encode(data, bits), zbase32);
        }
    }

    #[test]
    fn test_encode_superfluous_bits() {
        assert_eq!(encode(&[0xff, 0xff], 1), "o");
        assert_eq!(encode(&[0xd4, 0x7a, 0x04, 0xff], 24), "4t7ye");
    }

    #[test]
    #[should_panic(expected = "slice too short")]
    fn test_encode_short_slice() {
        encode(b"1234", 4 * 8 + 1);
    }

    #[test]
    fn test_encode_full_bytes() {
        for &(_, zbase32, data) in TEST_DATA.iter().filter(|&&(i, _, _)| i % 8 == 0) {
            assert_eq!(encode_full_bytes(data), zbase32);
        }
    }

    #[test]
    fn test_validate() {
        for &(_, zbase32, _) in TEST_DATA {
            assert!(validate(zbase32.as_bytes()));
            assert!(validate_str(zbase32));
        }
    }

    #[test]
    fn test_validate_invalid() {
        for string in INVALID_TEST_DATA.iter() {
            assert!(!validate(string.as_bytes()));
            assert!(!validate_str(string));
        }
    }

    #[test]
    fn test_valid_and_invalid_chars() {
        for char in 0..=255 {
            let bytes = &[char];
            if ALPHABET.contains(&char) {
                assert_eq!(ALPHABET[value_of_digit(char).unwrap() as usize], char);
                assert_eq!(encode(&decode(bytes, 5).unwrap(), 5).as_bytes(), bytes);
                assert!(validate(bytes));
            } else {
                assert!(value_of_digit(char).is_err());
                assert!(decode_full_bytes(bytes).is_err());
                assert!(!validate(bytes));
            }
        }
    }

    #[cfg(feature = "unstable")]
    #[bench]
    fn decode_one_mib(b: &mut test::Bencher) {
        let data = random_encoded_data(ONE_MIB);
        b.iter(|| decode_full_bytes(&data).unwrap())
    }

    #[cfg(feature = "unstable")]
    #[bench]
    fn encode_one_mib(b: &mut test::Bencher) {
        let data = random_data(ONE_MIB);
        b.iter(|| encode_full_bytes(&data))
    }

    #[cfg(feature = "unstable")]
    #[bench]
    fn decode_five_bytes(b: &mut test::Bencher) {
        let data = random_encoded_data(5);
        b.iter(|| decode_full_bytes(&data).unwrap())
    }

    #[cfg(feature = "unstable")]
    #[bench]
    fn encode_five_bytes(b: &mut test::Bencher) {
        let data = random_data(5);
        b.iter(|| encode_full_bytes(&data))
    }

    #[cfg(feature = "unstable")]
    #[bench]
    fn validate_one_mib(b: &mut test::Bencher) {
        let data = random_encoded_data(ONE_MIB);
        b.iter(|| validate(&data))
    }

    #[cfg(feature = "unstable")]
    fn random_data(bytes: usize) -> Vec<u8> {
        let mut data = vec![0_u8; bytes];
        rand::thread_rng().fill(&mut data[..]);
        data
    }

    #[cfg(feature = "unstable")]
    fn random_encoded_data(bytes: usize) -> Vec<u8> {
        let mut gen = rand::thread_rng();
        (0..bytes * 8 / 5)
            .map(|_| *ALPHABET.choose(&mut gen).unwrap())
            .collect()
    }
}

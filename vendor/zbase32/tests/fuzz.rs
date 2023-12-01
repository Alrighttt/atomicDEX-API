#[macro_use]
extern crate quickcheck;
extern crate zbase32;

mod common;
use common::*;

use quickcheck::TestResult;

quickcheck! {
    fn test_recode(data: Vec<u8>) -> bool {
        let encoded = zbase32::encode(&data, data.len() as u64 * 8);
        let encoded_bytes = zbase32::encode_full_bytes(&data);
        assert_eq!(encoded, encoded_bytes);

        let decoded = zbase32::decode(&encoded.as_bytes(), data.len() as u64 * 8).unwrap();
        let decoded_bytes = zbase32::decode_full_bytes(&encoded.as_bytes()).unwrap();
        assert_eq!(decoded, decoded_bytes);
        data == decoded
    }
}

quickcheck! {
    fn try_decode_ok(data: Vec<u8>) -> TestResult {
        if zbase32::validate(&data) {
            TestResult::from_bool(zbase32::decode_full_bytes(&data).is_ok())
        } else {
            TestResult::discard()
        }
    }
}

quickcheck! {
    fn try_decode_err(data: Vec<u8>) -> TestResult {
        if zbase32::validate(&data) {
            TestResult::discard()
        } else {
            TestResult::from_bool(zbase32::decode_full_bytes(&data).is_err())
        }
    }
}

quickcheck! {
    fn data_len_exceeds_bits_when_encoding(data: Vec<u8>, arbitrary: u8) -> TestResult {
        let len = data.len() as u64 * 8 + 1 + arbitrary as u64;
        TestResult::must_fail(move || {
            zbase32::encode(&data, len);
        })
    }
}

quickcheck! {
    fn data_len_exceeds_bits_when_ecoding(data: ZBaseEncodedData, arbitrary: u8) -> TestResult {
        let len = data.as_bytes().len() as u64 * 5 + 1 + arbitrary as u64;
        TestResult::must_fail(move || {
            let _ = zbase32::decode(data.as_bytes(), len);
        })
    }
}

quickcheck! {
    fn encode_too_long(data: Vec<u8>) -> () {
        let rand_bits = rand_bit_length(data.len(), 8);
        zbase32::encode(&data, rand_bits);
    }
}

quickcheck! {
    fn recode_partial(data: ZBaseEncodedData) -> bool {
        let rand_bits = rand_bit_length(data.len(), 5);
        let decoded1 = zbase32::decode(&data.as_bytes(), rand_bits).unwrap();
        let encoded = zbase32::encode(&decoded1, rand_bits);
        let decoded2 = zbase32::decode_str(&encoded, rand_bits).unwrap();
        decoded1 == decoded2
    }
}

quickcheck! {
    fn decode(data: ZBaseEncodedData) -> bool {
        zbase32::decode_full_bytes(&data.as_bytes()).is_ok()
    }
}

quickcheck! {
    fn validate(data: ZBaseEncodedData) -> bool {
        zbase32::validate(&data.as_bytes())
    }
}

quickcheck! {
    fn validate_str(data: ZBaseEncodedData) -> bool {
        let data = String::from_utf8(data.into_bytes()).unwrap();
        zbase32::validate_str(&data)
    }
}

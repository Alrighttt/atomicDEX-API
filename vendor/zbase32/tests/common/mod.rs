extern crate rand;
extern crate quickcheck;
extern crate zbase32;

use self::quickcheck::{Arbitrary, Gen};
use self::rand::seq::SliceRandom;
use self::rand::Rng;

#[derive(Clone, Debug)]
pub struct ZBaseEncodedData(Vec<u8>);

impl Arbitrary for ZBaseEncodedData {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let len = g.gen_range(0, 256);
        let content = (0..len)
            .map(|_| *zbase32::ALPHABET.choose(g).unwrap())
            .collect();
        ZBaseEncodedData(content)
    }
}

impl ZBaseEncodedData {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    #[allow(unused)]
    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

pub fn rand_bit_length(units: usize, bits_per_unit: u64) -> u64 {
    let bits = if units > 0 {
        rand::thread_rng().gen_range(0, units as u64 * bits_per_unit)
    } else {
        0
    };
    println!("random bit length: {}", bits);
    bits
}

use crate::utils::hasher::rapidhash::{rapidhash_known, rapidhash_nano};
use std::collections::HashMap;
use std::hash::{BuildHasher, Hash, Hasher};

/// reimplementation of rapidhash nano from [GitHub](https://github.com/Nicoshev/rapidhash/blob/master/rapidhash.h#L432)
/// (nano because were basically never gonna be hashing more than 48 bytes)
/// probably overkill but its really fast and i was bored.
#[derive(Default)]
pub struct RapidHasher {
    state: u64,
}

/// A deterministic HashMap using RapidHash.
///
/// This is so seeded RNG choosing will actually be seeded and not affected by the random hashing of normal hash maps.
///
/// This should also be faster than the default so maybe it could be used in place of the default for anythign that doesnt need the added security of the default hashing.
pub type DeterministicHashMap<K, V> = HashMap<K, V, RapidHasher>;

macro_rules! write_num {
    ($name: ident, $ty: ty) => {
        #[inline(always)]
        fn $name(&mut self, i: $ty) {
            self.state = rapidhash_known(&i.to_le_bytes())
        }
    };
}

impl Hasher for RapidHasher {
    #[inline(always)]
    fn finish(&self) -> u64 {
        self.state
    }

    #[inline(always)]
    fn write(&mut self, bytes: &[u8]) {
        self.state = rapidhash_nano(bytes)
    }

    write_num!(write_u8, u8);
    write_num!(write_u16, u16);
    write_num!(write_u32, u32);
    write_num!(write_u64, u64);
    write_num!(write_u128, u128);
    write_num!(write_usize, usize);
    write_num!(write_i8, i8);
    write_num!(write_i16, i16);
    write_num!(write_i32, i32);
    write_num!(write_i64, i64);
    write_num!(write_i128, i128);
    write_num!(write_isize, isize);
}

impl BuildHasher for RapidHasher {
    type Hasher = Self;

    fn build_hasher(&self) -> Self::Hasher {
        Self::default()
    }
}
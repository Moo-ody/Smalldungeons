use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

#[derive(Default)]
pub struct DeterministicHasher(u64);

/// A deterministic HashMap.
/// This is so seeded RNG choosing will actually be seeded and not affected by the random hashing of normal hash maps.
pub type DeterministicHashMap<K, V> = HashMap<K, V, BuildHasherDefault<DeterministicHasher>>;

impl Hasher for DeterministicHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = self.0.wrapping_mul(31).wrapping_add(b as u64);
        }
    }
}

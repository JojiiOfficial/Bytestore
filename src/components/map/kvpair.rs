use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq)]
pub struct KVPair<K, V> {
    key: K,
    value: V,
}

impl<K, V> KVPair<K, V> {
    #[inline]
    pub fn new(key: K, value: V) -> Self {
        Self { key, value }
    }

    #[inline]
    pub fn key(&self) -> &K {
        &self.key
    }

    #[inline]
    pub fn value(&self) -> &V {
        &self.value
    }

    #[inline]
    pub fn into_value(self) -> V {
        self.value
    }
}


impl<K, V> Into<(K, V)> for KVPair<K, V> {
    #[inline]
    fn into(self) -> (K, V) {
        (self.key, self.value)
    }
}
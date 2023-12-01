use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct IndexEntry {
    up_before: u64,
    index: u64,
}

impl IndexEntry {
    #[inline]
    pub fn new(up_before: u64, index: u64) -> Self {
        Self { up_before, index }
    }

    #[inline]
    pub fn up_before(&self) -> u64 {
        self.up_before
    }

    #[inline]
    pub fn index(&self) -> u64 {
        self.index
    }
}

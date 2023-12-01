use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Default, Serialize, Deserialize)]
pub struct BaseHeader {
    data_len: usize,
}

impl BaseHeader {
    pub fn new(data_len: usize) -> Self {
        BaseHeader { data_len }
    }

    #[inline]
    pub fn len_bytes() -> usize {
        8
    }

    #[inline]
    pub fn bytes(&self) -> [u8; 8] {
        self.data_len.to_le_bytes()
    }

    pub fn from_bytes(bytes: [u8; 8]) -> Self {
        Self::new(usize::from_le_bytes(bytes))
    }

    #[inline]
    pub fn data_len(&self) -> usize {
        self.data_len
    }

    #[inline]
    pub fn set_data_len(&mut self, len: usize) {
        self.data_len = len;
    }
}

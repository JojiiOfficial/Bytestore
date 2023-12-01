use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Copy, Clone)]
pub struct SplitFileHeader {
    split_pos: usize,
}

impl SplitFileHeader {
    #[inline]
    pub fn new(split_pos: usize) -> Self {
        Self { split_pos }
    }

    #[inline]
    pub fn split_pos(&self) -> usize {
        self.split_pos
    }
}

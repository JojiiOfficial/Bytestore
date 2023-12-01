use crate::components::list::cint::CompressionRatioWidth;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct CompressedIntListHeader {
    compression_ratio: CompressionRatioWidth,
    len: usize,
}

impl CompressedIntListHeader {
    #[inline]
    pub fn new(compression_ratio: CompressionRatioWidth, len: usize) -> Self {
        Self {
            compression_ratio,
            len,
        }
    }

    #[inline]
    pub fn compression_ratio(&self) -> CompressionRatioWidth {
        self.compression_ratio
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }
}

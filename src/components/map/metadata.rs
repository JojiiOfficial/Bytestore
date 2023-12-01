/// Metadata for a Hashmap.
#[derive(Default, Copy, Clone)]
pub struct MapMetadata {
    len: usize,
    capacity: usize,
}

impl MapMetadata {
    #[inline]
    pub fn new(len: usize, capacity: usize) -> Self {
        Self { len, capacity }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    #[inline]
    pub const fn byte_len() -> usize {
        8 + 8
    }

    /// Converts the Maps metadata to a byte array.
    #[inline]
    pub fn to_bytes(self) -> [u8; 16] {
        let mut bytes = [0u8; 16];
        bytes[..8].copy_from_slice(&self.len.to_le_bytes());
        bytes[8..].copy_from_slice(&self.capacity.to_le_bytes());
        bytes
    }

    /// Parses bytes to a MapMetadat
    #[inline]
    pub fn from_bytes(bytes: &[u8]) -> Self {
        assert_eq!(bytes.len(), Self::byte_len());
        let blen: [u8; 8] = unsafe { bytes[..8].try_into().unwrap_unchecked() };
        let bcap: [u8; 8] = unsafe { bytes[8..].try_into().unwrap_unchecked() };
        let len = usize::from_le_bytes(blen);
        let capacity = usize::from_le_bytes(bcap);
        Self { len, capacity }
    }
}

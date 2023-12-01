use crate::backend::memory::{MemoryBackend, MemoryData};
use crate::header::BaseHeader;
use crate::Result;

pub trait Creatable<B>
where
    Self: Sized,
{
    #[inline]
    fn create(backend: B) -> Result<Self> {
        Self::with_capacity(backend, 0)
    }

    fn with_capacity(backend: B, capacity: usize) -> Result<Self>;
}

pub trait MemCreatable: Creatable<MemoryBackend> {
    #[inline]
    fn create_mem() -> Result<Self> {
        Self::create_mem_with_capacity(8)
    }

    fn create_mem_with_capacity(capacity: usize) -> Result<Self> {
        // At least 4 bytes needed since `MemoryBackend` needs to store a `BaseHeader`!
        let capacity = capacity.max(BaseHeader::len_bytes());
        Self::create(MemoryBackend::from_storage(MemoryData::new(vec![
            0u8;
            capacity
        ]))?)
    }
}

impl<T: Creatable<MemoryBackend>> MemCreatable for T {}

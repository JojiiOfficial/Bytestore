use crate::backend::base::BaseBackend;
use crate::backend::growable::GrowableBackend;
use crate::backend::Backend;
use crate::error::Error;
use crate::header::BaseHeader;
use std::ops::{Deref, DerefMut};

pub type MemoryBackend = BaseBackend<MemoryData>;

pub struct MemoryData {
    data: Vec<u8>,
}

impl MemoryData {
    #[inline]
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
}

impl Backend for MemoryBackend {
    #[inline]
    fn data(&self) -> &[u8] {
        self.storage()
    }

    #[inline]
    fn data_mut(&mut self) -> &mut [u8] {
        self.storage_mut()
    }

    #[inline]
    fn first_index(&self) -> usize {
        BaseHeader::len_bytes()
    }

    #[inline]
    fn len(&self) -> usize {
        self.header().data_len()
    }

    fn set_len(&mut self, len: usize) -> Result<(), Error> {
        self.set_len(len)
    }

    fn replace(&mut self, index: usize, len: usize, data: &[u8]) -> Result<usize, Error> {
        let start = self.get_index(index);
        let end = start + len;

        self.check_capacity_oob(end)?;

        self.storage_mut()
            .data
            .splice(start..end, data.iter().copied());

        let diff = len.abs_diff(data.len());
        if len > data.len() {
            self.set_len(self.len() - diff)?;
        } else {
            self.set_len(self.len() + diff)?;
        }
        Ok(diff)
    }
}

impl GrowableBackend for MemoryBackend {
    #[inline]
    fn resize_impl(&mut self, new_size: usize, _: bool) -> crate::Result<()> {
        self.storage_mut().data.resize(new_size, 0u8);
        Ok(())
    }
}

impl Deref for MemoryData {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for MemoryData {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

#[cfg(test)]
pub mod test {
    use super::super::test::*;
    use super::*;

    pub fn make_deeta() -> impl Iterator<Item=String> {
        let mut i = 0;
        std::iter::from_fn(move || {
            let txt = format!("{i}_{i}_DATA").repeat(10 * i);
            i += 1;
            Some(txt)
        })
    }

    pub fn make_mem_backend(capacity: usize) -> MemoryBackend {
        MemoryBackend::from_storage(MemoryData::new(vec![0u8; capacity + 8])).unwrap()
    }

    #[test]
    fn memory_backend() {
        let mut backend = make_mem_backend(100);

        be_clear(&mut backend);
        be_replace(&mut backend);
        be_push(&mut backend);
        be_remove(&mut backend);
        be_fill(&mut backend);
    }

    #[test]
    fn sub_backend() {
        let mut big_backend = make_mem_backend(1024 * 1024);
        assert_eq!(big_backend.capacity(), 1024 * 1024);

        let sub = big_backend.sub_backend_mut(100).unwrap();
        assert_eq!(sub.capacity(), 100);
    }
}

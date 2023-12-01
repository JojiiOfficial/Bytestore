pub(super) mod entry;

use crate::backend::growable::GrowableBackend;
use crate::backend::Backend;
use crate::components::list::cint::index::entry::IndexEntry;
use crate::{Error, Result};

pub struct CIntListIndex<B> {
    backend: B,
}

impl<B> CIntListIndex<B> {
    #[inline]
    pub fn new(backend: B) -> Self {
        Self { backend }
    }
}

impl<B> CIntListIndex<B> where B: Backend {}

impl<B> CIntListIndex<B>
where
    B: GrowableBackend,
{
    pub fn insert(&mut self, entry: &IndexEntry) -> Result<()> {
        if self.backend.free() < 16 {
            self.backend.grow(16)?;
        }
        self.backend.push_t(entry)?;
        Ok(())
    }
}

impl<B> CIntListIndex<B>
where
    B: GrowableBackend,
{
    #[inline]
    pub fn get(&mut self, index: usize) -> Result<IndexEntry> {
        let start = index * 16;
        Ok(self.backend.get_t(start, 16)?)
    }

    /// Returns the last index entry.
    pub fn last(&mut self) -> Result<Option<IndexEntry>> {
        if self.backend.len() == 0 {
            return Ok(None);
        }

        let last = self
            .backend
            .len()
            .checked_sub(16)
            .ok_or(Error::OutOfBounds)?;
        Ok(Some(self.backend.get_t(last, 16)?))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::backend::memory::{MemoryBackend, MemoryData};
    use crate::traits::creatable::Creatable;
    use crate::Error;

    #[test]
    fn test_all() {
        let backend = MemoryBackend::create(MemoryData::new(vec![0u8; 8])).unwrap();
        let mut index = CIntListIndex::new(backend);
        let data = vec![
            IndexEntry::new(125, 2163),
            IndexEntry::new(1126325, 125),
            IndexEntry::new(23335, 9120),
        ];

        for i in data.iter() {
            index.insert(i).unwrap();
        }

        for (pos, i) in data.iter().enumerate() {
            assert_eq!(index.get(pos), Ok(*i));
        }

        assert_eq!(index.get(data.len()), Err(Error::OutOfBounds));
    }
}

mod header;
mod index;

use crate::backend::base::sub::BaseSubBackend;
use crate::backend::growable::GrowableBackend;
use crate::backend::Backend;
use crate::components::header_file::CustomHeaderFile;
use crate::components::list::cint::header::CompressedIntListHeader;
use crate::components::list::cint::index::CIntListIndex;
use crate::components::split_file::entry::Entry;
use crate::components::split_file::SplitFile;
use crate::traits::creatable::Creatable;
use crate::traits::initiable::Initiable;
use crate::Result;

type CompressionRatioWidth = u32;

pub const DEFAULT_COMPRESSION_RATIO: CompressionRatioWidth = 1000;

pub struct CompressedIntList<B> {
    backend: SplitFile<B>,
    compression_ratio: CompressionRatioWidth,
    len: usize,
}

impl<B> CompressedIntList<B>
where
    B: GrowableBackend,
{
    pub fn push<P>(&mut self, number: P) -> Result<()>
    where
        P: Into<u64>,
    {
        let number = number.into();
        let (encoded, enc_len) = varint_simd::encode(number);
        let encoded = &encoded[..enc_len as usize];

        if self.need_indexing() {
            self.add_index()?;
        }

        Ok(())
    }

    fn add_index(&mut self) -> Result<()> {
        Ok(())
    }

    /// Returns `true` if a new index entry needs to be inserted.
    fn need_indexing(&mut self) -> bool {
        debug_assert!(self.len > 0);

        let cr = self.compression_ratio as usize;
        if cr == 0 || self.len < cr {
            return false;
        }

        // Only need to index every n items where n = compression_ratio.
        (self.len - 1) % cr != 0
    }

    #[inline]
    fn index_mut(&mut self) -> CIntListIndex<CustomHeaderFile<Entry<B>, CompressionRatioWidth>> {
        let entry = self.backend.first_mut();
        let backend = CustomHeaderFile::init(entry).unwrap();
        CIntListIndex::new(backend)
    }

    #[inline]
    fn storage_mut(&mut self) -> Entry<B> {
        self.backend.second_mut()
    }
}

impl<B> CompressedIntList<B>
where
    B: Backend,
{
    pub fn get(&self, index: usize) -> Result<usize> {
        todo!()
    }

    #[inline]
    fn index(
        &self,
    ) -> CIntListIndex<CustomHeaderFile<BaseSubBackend<&[u8]>, CompressionRatioWidth>> {
        let entry = self.backend.first();
        let backend = CustomHeaderFile::init(entry).unwrap();
        CIntListIndex::new(backend)
    }

    #[inline]
    fn storage(&self) -> BaseSubBackend<&[u8]> {
        self.backend.second()
    }
}

impl<B> CompressedIntList<B>
where
    B: GrowableBackend,
{
    pub fn create_with_compression_ratio(
        backend: B,
        compression_ratio: CompressionRatioWidth,
    ) -> Result<Self> {
        let mut backend = SplitFile::create(backend).unwrap();
        let mut first = backend.first_mut();
        let header = CompressedIntListHeader::new(compression_ratio, 0);
        first.grow(12).unwrap();
        CustomHeaderFile::create(&mut first, header).unwrap();
        Ok(Self {
            backend,
            len: 0,
            compression_ratio,
        })
    }
}

impl<B> Creatable<B> for CompressedIntList<B>
where
    B: GrowableBackend,
{
    #[inline]
    fn with_capacity(backend: B, _: usize) -> Result<Self> {
        Self::create_with_compression_ratio(backend, DEFAULT_COMPRESSION_RATIO)
    }
}

impl<B> Initiable<B> for CompressedIntList<B>
where
    B: Backend,
{
    #[inline]
    fn init(backend: B) -> Result<Self> {
        let backend = SplitFile::init(backend)?;
        let header: CustomHeaderFile<_, CompressedIntListHeader> =
            CustomHeaderFile::init(backend.first())?;
        let header = header.header();
        let len = header.len();
        let compression_ratio = header.compression_ratio();
        Ok(Self {
            backend,
            len,
            compression_ratio,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::traits::creatable::MemCreatable;

    #[test]
    fn test_all() {
        let mut cil = CompressedIntList::create_mem_with_capacity(0).unwrap();
        cil.push(0u8).unwrap();
    }
}

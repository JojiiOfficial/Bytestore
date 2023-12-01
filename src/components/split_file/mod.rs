pub mod backend_index;
pub mod entry;
mod header;

use crate::backend::base::sub::BaseSubBackend;
use crate::backend::base::sub_mut::{BaseSubMutBackend, GeneralSubMutBackend};
use crate::backend::growable::GrowableBackend;
use crate::backend::Backend;
use crate::components::header_file::CustomHeaderFile;
use crate::components::split_file::entry::Entry;
use crate::error::Error;
use crate::header::BaseHeader;
use crate::traits::creatable::Creatable;
use crate::traits::initiable::Initiable;
use crate::traits::mtype::MType;
use crate::Result;
use backend_index::BackendIndex;
use header::SplitFileHeader;
use std::ops::Range;

/// A MType component that splits the backend into two separate backends that can work entirely independent
/// from each other. This also includes growing for both backends.
/// The `SplitFile` itself doesn't implement `Backend` since it provides two separate ones, not a single general one!
///
/// The internal format: (H_LEN (4 bytes) | HEADER (N bytes) | FIRST | SECOND)
pub struct SplitFile<B> {
    backend: CustomHeaderFile<B, SplitFileHeader>,
    header: [BaseHeader; 2],
}

impl<B> Creatable<B> for SplitFile<B>
    where
        B: GrowableBackend,
{
    #[inline]
    fn create(backend: B) -> Result<Self> {
        Self::with_capacity(backend, BaseHeader::len_bytes())
    }

    #[inline]
    fn with_capacity(backend: B, capacity: usize) -> Result<Self> {
        Self::create_with_init_cap(backend, capacity)
    }
}

impl<B> SplitFile<B>
    where
        B: GrowableBackend,
{
    pub fn create_with_init_cap(mut backend: B, init_cap: usize) -> Result<Self> {
        let need_total_capacity = 4 + 8 + (init_cap + BaseHeader::len_bytes()) * 2;
        if backend.capacity() < need_total_capacity {
            backend.grow(need_total_capacity - backend.capacity())?;
        }
        assert!(backend.capacity() >= need_total_capacity);

        let mut chf = CustomHeaderFile::create(backend, SplitFileHeader::default())?;

        let first_header = BaseHeader::new(0);
        chf.push(&first_header.bytes())?;
        // chf.push(&vec![0u8; init_cap])?;
        chf.push_fill(0, init_cap)?;

        let split_pos = chf.len();

        let second_header = BaseHeader::new(0);
        chf.push(&second_header.bytes())?;
        // chf.push(&vec![0u8; init_cap])?;
        chf.push_fill(0, init_cap)?;

        chf.set_header(SplitFileHeader::new(split_pos))?;

        let header = [first_header, second_header];

        chf.flush().unwrap();

        Ok(Self {
            backend: chf,
            header,
        })
    }

    /// Resizes the given backend.
    pub fn resize(&mut self, index: BackendIndex, delta: isize) -> Result<()> {
        if delta == 0 {
            return Ok(());
        }

        if delta > 0 {
            return self.grow(index, delta as usize);
        }

        self.shrink(index, delta.unsigned_abs())
    }

    /// Grows the given backend by the given size.
    pub fn grow(&mut self, index: BackendIndex, size: usize) -> Result<()> {
        // println!("Grow single: {index:?}");
        self.backend.grow(size)?;

        if index.is_first() {
            let split_pos = self.split_pos();
            // self.backend.replace(split_pos, 0, &vec![0u8; size])?;
            self.backend.replace_fill(split_pos, 0, 0, size)?;
            self.set_split_pos(split_pos + size)?;
        } else {
            // We use backend.len() as end for the second backend. Since we grew the backend a few lines
            // above, we can simply increase the length of it.
            self.backend.set_len(self.backend.len() + size)?;
        }

        Ok(())
    }

    /// Shrinks the given backend to delta bytes less.
    pub fn shrink(&mut self, index: BackendIndex, delta: usize) -> Result<()> {
        let be_range = self.backend_range(index);
        let header = self.get_header_for(index);
        let new_len = be_range
            .len()
            .checked_sub(delta)
            .ok_or(Error::OutOfBounds)?;

        if new_len < header.data_len() + BaseHeader::len_bytes() {
            return Err(Error::OutOfBounds);
        }

        if index.is_first() {
            self.backend.replace(new_len, delta, &[])?;
            self.set_split_pos(self.split_pos() - delta)?;
        }

        self.backend.shrink(delta)?;
        Ok(())
    }

    pub fn shrink_to_fit(&mut self) -> Result<()> {
        self.backend.shrink_to_fit()
    }

    pub fn grow_both(&mut self, first_size: usize, second_size: usize) -> Result<()> {
        // println!("Grow both");

        let total_grow = first_size + second_size;
        self.backend.grow(total_grow)?;

        if first_size > 0 {
            let split_pos = self.split_pos();
            // self.backend.replace(split_pos, 0, &vec![0u8; first_size])?;
            self.backend.replace_fill(split_pos, 0, 0, first_size)?;
            self.set_split_pos(split_pos + first_size)?;
        }

        if second_size > 0 {
            // We use backend.len() as end for the second backend. Since we grew the backend a few lines
            // above, we can simply increase the length of it.
            self.backend.set_len(self.backend.len() + second_size)?;
        }

        Ok(())
    }
}

impl<'b, B> SplitFile<B>
    where
        B: Backend + 'b,
{
    /// Initializes a `SplitFile` for a given backend that already contains a created SplitFile.
    pub fn init(backend: B) -> Result<Self> {
        let chf: CustomHeaderFile<B, SplitFileHeader> = CustomHeaderFile::init(backend)?;
        let split_pos = chf.header().split_pos();

        let first_header = chf.get(0, BaseHeader::len_bytes())?;
        let first_header = BaseHeader::from_bytes(first_header.try_into().unwrap());

        let second_header = chf.get(split_pos, BaseHeader::len_bytes())?;
        let second_header = BaseHeader::from_bytes(second_header.try_into().unwrap());

        Ok(Self {
            backend: chf,
            header: [first_header, second_header],
        })
    }

    /// Returns the first backend.
    #[inline]
    pub fn first(&self) -> BaseSubBackend<&[u8]> {
        self.get_backend(BackendIndex::First)
    }

    /// Returns the first backend mutable.
    #[inline]
    pub fn first_mut(&mut self) -> Entry<B> {
        self.entry_mut(BackendIndex::First)
    }

    /// Returns the second backend.
    #[inline]
    pub fn second(&self) -> BaseSubBackend<&[u8]> {
        self.get_backend(BackendIndex::Second)
    }

    /// Returns the second backend mutable.
    #[inline]
    pub fn second_mut(&mut self) -> Entry<B> {
        self.entry_mut(BackendIndex::Second)
    }

    /// Returns an `Entry` for the given BackendIndex.
    #[inline]
    pub fn entry_mut(&mut self, index: BackendIndex) -> Entry<B> {
        Entry::new(self, index)
    }

    /// Gets both backends mutable (yes its possible :P even in safe rust).
    pub fn both_mut(&mut self) -> (GeneralSubMutBackend, GeneralSubMutBackend) {
        let fstart = self.backend.get_index(0);
        let fend = self.backend.get_index(self.split_pos());
        let send = self.backend.get_index(self.backend.len());

        let (hf, hs) = self.header.split_at_mut(1);

        let (fd, sd) = self.backend.data_mut()[fstart..send].split_at_mut(fend - fstart);

        let first = BaseSubMutBackend::new(fd, &mut hf[0]);
        let second = BaseSubMutBackend::new(sd, &mut hs[0]);

        (first, second)
    }

    /// Returns the given backend mutable.
    pub fn get_backend_mut(&mut self, index: BackendIndex) -> GeneralSubMutBackend {
        let be_range = self.backend_range(index);
        let header = &mut self.header[index as usize - 1];
        let data = &mut self.backend.data_mut()[be_range];
        GeneralSubMutBackend::new(data, header)
    }

    /// Returns the given backend.
    pub fn get_backend(&self, index: BackendIndex) -> BaseSubBackend<&[u8]> {
        let be_range = self.backend_range(index);
        let header = self.get_header_for(index);
        let data = &self.backend.data()[be_range];
        BaseSubBackend::new(data, header)
    }

    #[inline]
    pub fn flush(&mut self) -> Result<()> {
        self.backend.flush()
    }

    /// Returns the raw backend data for the given Backend.
    #[inline]
    pub fn backend_data(&self, index: BackendIndex) -> &[u8] {
        let be_range = self.backend_range(index);
        &self.backend.data()[be_range]
    }

    /// Returns the raw backend data for the given Backend mutable.
    #[inline]
    pub fn backend_data_mut(&mut self, index: BackendIndex) -> &mut [u8] {
        let be_range = self.backend_range(index);
        &mut self.backend.data_mut()[be_range]
    }

    /// Flushes the given backend.
    pub fn flush_backend(&mut self, index: BackendIndex) -> Result<()> {
        let be_range = self.backend_range(index);
        self.backend.flush_range(be_range.start, be_range.len())
    }

    /// Flushes a given range of a backend
    pub fn flush_backend_range(
        &mut self,
        index: BackendIndex,
        start: usize,
        len: usize,
    ) -> Result<()> {
        let be_range = self.backend_range(index);
        let start = be_range.start + start;
        self.backend.flush_range(start, len)
    }

    /// Gets the index range of data in the wrapping backend for a given splitted backend.
    fn backend_range(&self, index: BackendIndex) -> Range<usize> {
        let split_pos = self.split_pos();

        let (start, end) = match index {
            BackendIndex::First => (0, split_pos),
            BackendIndex::Second => (split_pos, self.backend.len()),
        };

        let start = self.backend.get_index(start);
        let end = self.backend.get_index(end);

        start..end
    }

    /// Changes the position to split the two files.
    fn set_split_pos(&mut self, pos: usize) -> Result<()> {
        if pos > self.backend.capacity() {
            return Err(Error::OutOfBounds);
        }

        let new_header = SplitFileHeader::new(pos);
        self.backend.set_header(new_header)?;
        Ok(())
    }
}

impl<B> SplitFile<B> {
    /// Returns the split position (index) of the both backends. This is the value of the first byte
    /// in the second backend to be more precise.
    #[inline]
    fn split_pos(&self) -> usize {
        self.backend.header().split_pos()
    }

    #[inline]
    pub(crate) fn get_header_for(&self, index: BackendIndex) -> &BaseHeader {
        &self.header[index as usize - 1]
    }
}

impl<B> MType for SplitFile<B>
    where
        B: Backend,
{
    #[inline]
    fn raw_data(&self) -> &[u8] {
        self.backend.data()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::backend::memory::test::make_mem_backend;
    use crate::backend::mmap_mut::test::make_mmap_backend;

    #[test]
    fn test_init_cap() {
        for i in 0..50 {
            for init_cap in i..50 {
                let be = make_mem_backend(i);

                let sf = SplitFile::create_with_init_cap(be, init_cap).unwrap();
                assert_eq!(sf.first().capacity(), init_cap);
                assert_eq!(sf.second().capacity(), init_cap);
                assert_eq!(
                    sf.backend.capacity(),
                    (init_cap + BaseHeader::len_bytes()) * 2
                );
            }
        }
    }

    #[test]
    fn test_all() {
        let mut mem = make_mem_backend(8);
        test_shrink(&mut mem);

        let mut mem = make_mem_backend(8);
        test_small_grow(&mut mem);
        test_grow(&mut mem);
        test_split_file(&mut mem);
        test_init(&mut mem);
        test_both_mut(&mut mem);

        let mut file = make_mmap_backend("./growme", 8);
        test_shrink(&mut file);

        let mut file = make_mmap_backend("./growme2", 8);
        test_small_grow(&mut file);
        test_grow(&mut file);
        test_split_file(&mut file);
        test_init(&mut file);
        test_both_mut(&mut mem);
    }

    fn test_small_grow<B: GrowableBackend>(be: &mut B) {
        assert!(be.capacity() < 10);

        let mut sf = SplitFile::create_with_init_cap(be, 1).unwrap();
        assert_eq!(sf.first().capacity(), 1);

        for i in 0..1000 {
            if i % 2 == 0 {
                if sf.first().free() < 4 {
                    sf.grow(BackendIndex::First, 4).unwrap();
                }
                let pos = sf.first_mut().push(&[3, 1, 1, 3]).unwrap();
                assert_eq!(sf.first().get(pos, 4), Ok(&[3, 1, 1, 3][..]));
            }

            if sf.second().free() < 4 {
                sf.grow(BackendIndex::Second, 4).unwrap();
            }
            let pos = sf.second_mut().push(&[1, 2, 3, 4]).unwrap();
            assert_eq!(sf.second().get(pos, 4), Ok(&[1, 2, 3, 4][..]));
        }
    }

    fn test_split_file<B: GrowableBackend>(be: &mut B) {
        be.clear();
        let mut sf = SplitFile::create(be).unwrap();
        assert!(sf.first_mut().capacity() >= 1);
        assert!(sf.second_mut().capacity() >= 1);

        assert_eq!(sf.first_mut().get(0, 1), Err(Error::OutOfBounds));
        sf.first_mut().push(&[5]).unwrap();
        assert_eq!(sf.first_mut().get(0, 1), Ok(&[5][..]));

        assert_eq!(sf.second_mut().get(0, 1), Err(Error::OutOfBounds));
        sf.second_mut().push(&[9]).unwrap();
        assert_eq!(sf.second_mut().get(0, 1), Ok(&[9][..]));

        assert_eq!(sf.first_mut().get(0, 1), Ok(&[5][..]));
    }

    fn test_init<B: GrowableBackend>(mut be: &mut B) {
        be.clear();
        let mut sf = SplitFile::create_with_init_cap(&mut be, 10).unwrap();
        let old_first_cap = sf.first_mut().capacity();
        let old_sec_cap = sf.second_mut().capacity();
        assert!(sf.first_mut().capacity() >= 10);
        assert!(sf.second_mut().capacity() >= 10);

        sf.first_mut().push(&[3]).unwrap();
        sf.second_mut().push(&[4]).unwrap();
        assert_eq!(sf.first_mut().get(0, 1), Ok(&[3][..]));
        assert_eq!(sf.second_mut().get(0, 1), Ok(&[4][..]));

        let mut sf = SplitFile::init(be).unwrap();
        assert_eq!(sf.first_mut().get(0, 1), Ok(&[3][..]));
        assert_eq!(sf.second_mut().get(0, 1), Ok(&[4][..]));

        sf.grow(BackendIndex::First, 3).unwrap();
        assert_eq!(sf.first_mut().capacity(), old_first_cap + 3);
        assert_eq!(sf.second_mut().capacity(), old_sec_cap);

        sf.grow(BackendIndex::Second, 3).unwrap();
        assert_eq!(sf.first_mut().capacity(), old_first_cap + 3);
        assert_eq!(sf.second_mut().capacity(), old_sec_cap + 3);
    }

    fn test_both_mut<B: GrowableBackend>(be: &mut B) {
        be.clear();
        let mut sf = SplitFile::create_with_init_cap(be, 20).unwrap();

        sf.first_mut().push(&[1, 2, 3]).unwrap();
        sf.first_mut().push(&[9, 8, 7]).unwrap();

        sf.second_mut().push(&[5, 4, 5]).unwrap();
        sf.second_mut().push(&[12, 7, 21]).unwrap();
        sf.second_mut().push(&[1, 4, 4, 9]).unwrap();

        let f_data = sf.first().data().to_vec();
        let s_data = sf.second().data().to_vec();

        let (f, s) = sf.both_mut();
        assert_eq!(f.data(), f_data);
        assert_eq!(s.data(), s_data);
        // assert_eq!(f.len(), 2);
        // assert_eq!(s.len(), 3);
    }

    fn test_grow<B: GrowableBackend>(be: &mut B) {
        be.clear();
        let mut sf = SplitFile::create_with_init_cap(be, 1).unwrap();
        assert!(sf.first().capacity() >= 1);
        assert!(sf.second().capacity() >= 1);

        sf.first_mut().push(&[3]).unwrap();
        sf.second_mut().push(&[9]).unwrap();

        assert_eq!(sf.first().get(0, 1), Ok(&[3][..]));
        assert_eq!(sf.second().get(0, 1), Ok(&[9][..]));
        assert_eq!(sf.first().len(), 1);
        assert_eq!(sf.second().len(), 1);

        let old_len = sf.backend.len();
        let old_cap = sf.backend.capacity();
        sf.grow(BackendIndex::First, 2).unwrap();
        assert_eq!(sf.backend.len(), old_len + 2);
        assert_eq!(sf.backend.capacity(), old_cap + 2);
        assert_eq!(sf.first().capacity(), 3);

        assert_eq!(sf.first().len(), 1);
        assert_eq!(sf.second().len(), 1);

        assert_eq!(sf.first().get(0, 1), Ok(&[3][..]));
        assert_eq!(sf.second().get(0, 1), Ok(&[9][..]));

        sf.grow(BackendIndex::Second, 2).unwrap();
        assert_eq!(sf.first().get(0, 1), Ok(&[3][..]));
        assert_eq!(sf.second().get(0, 1), Ok(&[9][..]));
        assert_eq!(sf.first().capacity(), 3);
        assert!(sf.second().capacity() >= 3);
        assert_eq!(sf.first().len(), 1);
        assert_eq!(sf.second().len(), 1);

        sf.grow(BackendIndex::First, 13).unwrap();
        assert_eq!(sf.first().capacity(), 16);
        assert_eq!(sf.first().get(0, 1), Ok(&[3][..]));
        assert_eq!(sf.second().get(0, 1), Ok(&[9][..]));

        sf.first_mut().push(&[31, 12, 45]).unwrap();
        assert_eq!(sf.first().get(0, 1), Ok(&[3][..]));
        assert_eq!(sf.first().get(1, 3), Ok(&[31, 12, 45][..]));
        assert_eq!(sf.first().len(), 4);

        assert_eq!(sf.second().get(0, 1), Ok(&[9][..]));
        assert_eq!(sf.second().len(), 1);
    }

    fn test_shrink<B: GrowableBackend>(be: &mut B) {
        be.clear();
        let mut sf = SplitFile::create_with_init_cap(be, 10).unwrap();
        assert_eq!(sf.first().capacity(), 10);
        assert_eq!(sf.second().capacity(), 10);

        sf.shrink(BackendIndex::First, 2).unwrap();
        assert_eq!(sf.first().capacity(), 8);
        assert_eq!(sf.second().capacity(), 10);

        sf.entry_mut(BackendIndex::First)
            .push(&[1, 2, 3, 9])
            .unwrap();
        sf.entry_mut(BackendIndex::Second)
            .push(&[9, 9, 9, 8])
            .unwrap();
        assert_eq!(sf.first().get(0, 4), Ok(&[1, 2, 3, 9][..]));
        assert_eq!(sf.second().get(0, 4), Ok(&[9, 9, 9, 8][..]));
        assert_eq!(sf.first().data()[8..].len(), 8);

        sf.shrink(BackendIndex::First, 4).unwrap();
        assert_eq!(sf.first().capacity(), 4);
        assert_eq!(sf.first().len(), 4);
        assert!(sf.first().is_full());
        assert_eq!(sf.first().get(0, 4), Ok(&[1, 2, 3, 9][..]));
        assert_eq!(sf.second().get(0, 4), Ok(&[9, 9, 9, 8][..]));

        let mut exp_data = vec![];
        exp_data.extend_from_slice(sf.backend_data(BackendIndex::First));
        exp_data.extend_from_slice(sf.backend_data(BackendIndex::Second));
        assert_eq!(&sf.backend.data()[sf.backend.get_index(0)..], &exp_data);

        assert_eq!(&sf.first().data()[8..], &[1, 2, 3, 9][..]);
        assert_eq!(
            &sf.backend_data(BackendIndex::First)[8..],
            &[1, 2, 3, 9][..]
        );
        assert_eq!(sf.first().data()[8..].len(), 4);

        assert_eq!(sf.shrink(BackendIndex::First, 1), Err(Error::OutOfBounds));
        assert_eq!(sf.shrink(BackendIndex::Second, 1), Err(Error::OutOfBounds));
    }
}

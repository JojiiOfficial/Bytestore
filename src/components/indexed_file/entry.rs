use crate::backend::growable::GrowableBackend;
use crate::backend::Backend;
use crate::components::indexed_file::IndexedFile;
use crate::components::split_file::backend_index::BackendIndex;
use crate::error::Error;
use crate::header::BaseHeader;
use crate::traits::mtype::MType;
use serde::Serialize;
use std::fmt::{Debug, Formatter};
use std::ops::{Deref, DerefMut, Range};

/// A single entry of an `IndexedFile`. Implements [`Backend`] and [`GrowableBackend`].
pub struct Entry<'d, B> {
    ifile: &'d mut IndexedFile<B>,
    id: usize,
    index: Range<usize>,
}

impl<'d, B> Entry<'d, B> {
    #[inline]
    pub(super) fn new(ifile: &'d mut IndexedFile<B>, id: usize, index: Range<usize>) -> Self {
        Self { ifile, id, index }
    }
}

impl<'d, B> Entry<'d, B>
    where
        B: GrowableBackend,
{
    #[inline]
    pub fn grow_with_data(&mut self, data: &[u8]) -> Result<(), Error> {
        self.ifile.grow_entry_with_data(self.id, data)
    }

    /// Replaces the entries data to `item` serialized. If the new item serialized is longer than
    /// the current data, the entry will automatically grow.
    #[inline]
    pub fn set_t<T: Serialize>(&mut self, item: &T) -> Result<isize, Error> {
        self.set(&crate::deser::serialize_impl(item)?)
    }

    /// Replaces the entries data to `new_data`. If `new_data` is empty, the entries data will shrink
    /// to 0 and if new_data is longer than the current data, the entry will automatically grow.
    pub fn set(&mut self, new_data: &[u8]) -> Result<isize, Error> {
        let diff = new_data.len() as isize - self.len() as isize;

        if diff < 0 {
            self.set_shrinking(new_data, diff)?;
        } else {
            self.set_growing(new_data, diff)?
        }

        Ok(diff)
    }

    /// Sets the entries data to `new_data` where the new length is equal or exceeds the current entries length.
    fn set_growing(&mut self, new_data: &[u8], diff: isize) -> Result<(), Error> {
        debug_assert!(diff >= 0);

        if diff > 0 {
            self.grow(diff as usize)?;
        }

        self.ifile
            .backend
            .second_mut()
            // We've already grewn the Item so the current entry already has the same length as `new_data`
            .replace_same_len(self.index.start - BaseHeader::len_bytes(), new_data)?;

        Ok(())
    }
}

impl<'d, B> Entry<'d, B>
    where
        B: Backend,
{
    /// Sets the entries data to `new_data` where the new length is less than the current length.
    fn set_shrinking(&mut self, new_data: &[u8], diff: isize) -> Result<(), Error> {
        debug_assert!(diff < 0);

        let len = self.len();

        self.ifile
            .backend
            .second_mut()
            .replace(self.index.start - BaseHeader::len_bytes(), len, new_data)
            .unwrap();

        self.index.end = (self.index.end as isize + diff) as usize;
        self.ifile.shift_offsets(self.id, diff)?;
        Ok(())
    }
}

impl<'d, B> GrowableBackend for Entry<'d, B>
    where
        B: GrowableBackend,
{
    fn resize_impl(&mut self, _new_size: usize, _growing: bool) -> crate::Result<()> {
        panic!("Don't call resize_impl directly");
    }

    fn grow(&mut self, size: usize) -> crate::Result<()> {
        self.ifile.grow_entry(self.id, size, 0)?;
        self.index.end += size;
        Ok(())
    }

    fn shrink(&mut self, size: usize) -> crate::Result<()> {
        self.ifile.shrink_entry_unchecked(self.id, size)?;
        self.index.end -= size;
        Ok(())
    }
}

impl<'d, B> Backend for Entry<'d, B>
    where
        B: Backend,
{
    #[inline]
    fn data(&self) -> &[u8] {
        &self.ifile.backend.backend_data(BackendIndex::Second)[self.index.clone()]
    }

    #[inline]
    fn data_mut(&mut self) -> &mut [u8] {
        &mut self.ifile.backend.backend_data_mut(BackendIndex::Second)[self.index.clone()]
    }

    #[inline]
    fn first_index(&self) -> usize {
        0
    }

    #[inline]
    fn len(&self) -> usize {
        self.index.len()
    }

    fn set_len(&mut self, _len: usize) -> Result<(), Error> {
        // Len is always the length of the underlying data.
        Ok(())
    }

    #[inline]
    fn clear(&mut self) {
        self.set_shrinking(&[], -(self.len() as isize)).unwrap();
    }
}

impl<'d, B> Debug for Entry<'d, B>
    where
        B: Backend,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Entry")
            .field("id", &self.id)
            .field("length", &self.len())
            .finish()
    }
}

impl<'d, B> Deref for Entry<'d, B>
    where
        B: Backend,
{
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.data()
    }
}

impl<'d, B> DerefMut for Entry<'d, B>
    where
        B: Backend,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data_mut()
    }
}

impl<'d, B> MType for Entry<'d, B>
    where
        B: Backend,
{
    #[inline]
    fn raw_data(&self) -> &[u8] {
        self.data()
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::backend::memory::test::make_mem_backend;
    use crate::backend::mmap_mut::test::make_mmap_backend;
    use crate::traits::creatable::Creatable;

    pub const TEST_DATA_1: &[u8] = &[1, 2, 3, 4, 99, 5, 1];
    pub const TEST_DATA_2: &[u8] = &[6, 9, 7, 9, 31];
    pub const TEST_DATA_3: &[u8] = "myTestDatAabcあれa0".as_bytes();
    pub const TEST_DATA_4: &[u8] = "777あれ0yTa12AUEO音楽が".as_bytes();

    pub const ALL_TEST_DATA: &[&[u8]] = &[TEST_DATA_1, TEST_DATA_2, TEST_DATA_3, TEST_DATA_4];

    pub fn insert_test_data<B: GrowableBackend>(ifile: &mut IndexedFile<B>) {
        ifile.clear();
        let id = ifile.insert(TEST_DATA_1).unwrap();
        assert_eq!(id, 0);
        let id = ifile.insert(TEST_DATA_2).unwrap();
        assert_eq!(id, 1);
        let id = ifile.insert(TEST_DATA_3).unwrap();
        assert_eq!(id, 2);
        let id = ifile.insert(TEST_DATA_4).unwrap();
        assert_eq!(id, 3);
        assert_eq!(ifile.count, 4);

        check_test_data(ifile, 0);
        /*
        assert_eq!(ifile.get(0), Ok(TEST_DATA_1));
        assert_eq!(ifile.get(1), Ok(TEST_DATA_2));
        assert_eq!(ifile.get(2), Ok(TEST_DATA_3));
        assert_eq!(ifile.get(3), Ok(TEST_DATA_4));
         */
    }

    pub fn check_test_data<B: Backend>(ifile: &IndexedFile<B>, start_index: usize) {
        let end = ALL_TEST_DATA.len();

        for i in start_index..end {
            assert_eq!(ifile.get(i), Ok(ALL_TEST_DATA[i]));
        }
    }

    #[test]
    fn test_all() {
        let backend = make_mem_backend(50);
        let mut ifile = IndexedFile::create(backend).unwrap();
        test_entry_simple(&mut ifile);
        test_entry_clear(&mut ifile);
        test_set_value(&mut ifile);

        let backend = make_mmap_backend("./entry", 50);
        let mut ifile = IndexedFile::create(backend).unwrap();
        test_entry_simple(&mut ifile);
        test_entry_clear(&mut ifile);
        test_set_value(&mut ifile);
    }

    fn test_set_value<B: GrowableBackend>(ifile: &mut IndexedFile<B>) {
        ifile.clear();
        insert_test_data(ifile);

        ifile.entry(1).unwrap().set(TEST_DATA_1).unwrap();

        assert_eq!(ifile.entry(0).unwrap().data(), TEST_DATA_1);
        assert_eq!(ifile.entry(1).unwrap().data(), TEST_DATA_1);
        assert_eq!(ifile.entry(2).unwrap().data(), TEST_DATA_3);

        ifile.entry(1).unwrap().set(TEST_DATA_3).unwrap();

        assert_eq!(ifile.entry(0).unwrap().data(), TEST_DATA_1);
        assert_eq!(ifile.entry(1).unwrap().data(), TEST_DATA_3);
        assert_eq!(ifile.entry(2).unwrap().data(), TEST_DATA_3);
    }

    fn test_entry_simple<B: GrowableBackend>(ifile: &mut IndexedFile<B>) {
        ifile.clear();
        insert_test_data(ifile);

        ifile.entry(0).unwrap().grow(10).unwrap();

        let mut new_td_1: Vec<_> = TEST_DATA_1.to_vec();
        new_td_1.extend((0..10).map(|_| 0));

        assert_eq!(ifile.entry(0).unwrap().data(), &new_td_1);
        assert_eq!(ifile.entry(1).unwrap().data(), TEST_DATA_2);
        assert_eq!(ifile.entry(2).unwrap().data(), TEST_DATA_3);

        ifile.entry(1).unwrap().grow(13).unwrap();
        assert_eq!(ifile.entry(2).unwrap().data(), TEST_DATA_3);
        assert_eq!(ifile.entry(0).unwrap().data(), &new_td_1);

        let mut new_td_2: Vec<_> = TEST_DATA_2.to_vec();
        new_td_2.extend((0..13).map(|_| 0));
        assert_eq!(ifile.entry(1).unwrap().data(), &new_td_2);
    }

    fn test_entry_clear<B: GrowableBackend>(ifile: &mut IndexedFile<B>) {
        ifile.clear();
        insert_test_data(ifile);

        ifile.entry(0).unwrap().clear();
        assert_eq!(ifile.entry(0).unwrap().data(), &[]);
        assert_eq!(ifile.entry(1).unwrap().data(), TEST_DATA_2);
        assert_eq!(ifile.entry(2).unwrap().data(), TEST_DATA_3);
    }
}

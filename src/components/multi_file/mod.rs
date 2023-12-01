pub mod entry_mut;

use crate::backend::base::sub::BaseSubBackend;
use crate::backend::base::sub_mut::{BaseSubMutBackend, GeneralSubMutBackend};
use crate::backend::base::BaseBackend;
use crate::backend::growable::GrowableBackend;
use crate::backend::Backend;
use crate::components::indexed_file::IndexedFile;
use crate::components::multi_file::entry_mut::MFileEntryMut;
use crate::header::BaseHeader;
use crate::traits::creatable::Creatable;
use crate::traits::initiable::Initiable;
use crate::traits::mtype::MType;
use crate::{Error, Result};
use mult_split::MultiSplit;

/// Similar to `SplitFile<B>` but supports n different sub-backends encoded in a single file.
pub struct MultiFile<B> {
    backend: IndexedFile<B>,
    headers: Vec<BaseHeader>,
}

impl<B> MultiFile<B>
    where
        B: Backend,
{
    /// Gets a Backend by its ID.
    pub fn get(&self, id: usize) -> Option<BaseSubBackend<&[u8]>> {
        let data = self.backend.get(id).ok()?;
        let header = self.header(id)?;
        Some(BaseSubBackend::new(data, header))
    }

    /// Gets a Backend mutable by its ID.
    pub fn get_mut(&mut self, id: usize) -> Option<GeneralSubMutBackend> {
        let data = self.backend.get_mut(id).ok()?;
        let header = self.headers.get_mut(id)?;
        Some(BaseSubMutBackend::new(data, header))
    }

    /// Gets multiple items by their indices mutable at the same time. Returns `None` if at least one index is not an index
    /// in the multi file.
    pub fn get_n_by_index_mut<const N: usize>(
        &mut self,
        indices: [usize; N],
    ) -> Option<[GeneralSubMutBackend; N]> {
        if indices.iter().any(|i| !self.has_id(*i)) {
            return None;
        }

        let backends: [&mut [u8]; N] = self.backend.get_n_by_index(indices)?;
        let mut header_split = MultiSplit::new(&mut self.headers);

        let mut i = 0;
        let res = backends.map(|backend| {
            let hindex = indices[i];
            let header = &mut header_split.borrow_mut(hindex..hindex + 1).unwrap()[0];
            i += 1;
            BaseSubMutBackend::new(backend, header)
        });
        Some(res)
    }

    /// Gets two backends by their ID mutable. For getting more than two backends mutable see `get_n_by_index_mut`.
    #[inline]
    pub fn get_two_mut(
        &mut self,
        first: usize,
        second: usize,
    ) -> Result<(GeneralSubMutBackend, GeneralSubMutBackend)> {
        Ok(self
            .get_n_by_index_mut([first, second])
            .ok_or(Error::OutOfBounds)?
            .into())
    }

    /// Retruns the `MFileEntry` for the given ID.
    pub fn entry_mut(&mut self, id: usize) -> Option<MFileEntryMut<B>> {
        if !self.has_id(id) {
            return None;
        }
        Some(MFileEntryMut::new(self, id))
    }

    /// Returns an entry and initializes a new backend with the entries data. This can be used together
    /// with `insert_new_backend` to use a single MultiFile to hold multiple backends.
    pub fn get_backend_mut<'a, E>(&'a mut self, id: usize) -> Option<E>
        where
            E: Initiable<MFileEntryMut<'a, B>>,
    {
        let entry = self.entry_mut(id)?;
        E::init(entry).ok()
    }

    /// with `insert_new_backend` to use a single MultiFile to hold multiple backends.
    pub fn get_backend<'a, E>(&'a self, id: usize) -> Option<E>
        where
            E: Initiable<BaseSubBackend<'a, &'a [u8]>>,
    {
        let entry = self.get(id)?;
        E::init(entry).ok()
    }

    #[inline]
    pub fn has_id(&self, id: usize) -> bool {
        self.backend.has_id(id)
    }

    /// Removes all backends from the `MultiFile` but keeps the allocated size.
    pub fn clear(&mut self) {
        self.backend.clear();
        self.headers.clear();
    }

    /// Flushes all entries.
    #[inline]
    pub fn flush(&mut self) -> Result<()> {
        self.backend.flush()
    }

    /// Flushes a single entry.
    #[inline]
    pub fn flush_entry(&mut self, id: usize) -> Result<()> {
        self.backend.flush_item(id)
    }

    #[inline]
    pub(super) fn get_be_data_mut(&mut self, id: usize) -> Result<&mut [u8]> {
        self.backend.get_mut(id)
    }
    #[inline]
    pub(super) fn get_be_data(&self, id: usize) -> Result<&[u8]> {
        self.backend.get(id)
    }
}

impl<B: GrowableBackend> MultiFile<B> {
    /// Inserts a new backend type into the MultiFile. This must start with a BaseHeader.
    pub fn insert<T>(&mut self, item: T) -> Result<usize>
        where
            T: MType,
    {
        let data = item.raw_data();
        let backend = BaseBackend::from_storage(data)?;
        let id = self.backend.insert(data)?;
        self.headers.push(*backend.header());
        Ok(id)
    }

    /// Creates and inserts a new empty backend of type `E` and returns it. This works on the underlying
    /// `MFileEntry` as backend so writing a backend returned by this function changes the `MFile`s entry.
    pub fn insert_new_backend<'a, E>(&'a mut self) -> Result<E>
        where
            E: Creatable<MFileEntryMut<'a, B>>,
    {
        E::create(self.insert_empty()?)
    }

    /// Inserts a new item containing a default base header.
    pub fn insert_empty(&mut self) -> Result<MFileEntryMut<B>> {
        let empty = &mut BaseHeader::new(0).bytes()[..];
        let backend = BaseBackend::from_storage(empty)?;
        let id = self.backend.insert(backend.data())?;
        self.headers.push(*backend.header());
        Ok(MFileEntryMut::new(self, id))
    }

    #[inline]
    pub(super) fn grow(&mut self, id: usize, size: usize) -> Result<()> {
        self.backend.grow_entry(id, size, 0)
    }

    #[inline]
    pub fn shrink_to_fit(&mut self) -> Result<()> {
        self.backend.shrink_to_fit()
    }

    #[inline]
    pub(super) fn shrink(&mut self, id: usize, size: usize) -> Result<()> {
        self.backend.shrink_entry_unchecked(id, size)
    }
}

impl<B> MultiFile<B> {
    /// Returns the amoutn of sub-backends the `MultiFile` holds.
    #[inline]
    pub fn count(&self) -> usize {
        self.headers.len()
    }

    /// Returns `true` if the MultiFile is empty and doesn't hold any data.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.headers.is_empty()
    }

    #[inline]
    fn header(&self, id: usize) -> Option<&BaseHeader> {
        self.headers.get(id)
    }
}

impl<B: GrowableBackend> Creatable<B> for MultiFile<B> {
    #[inline]
    fn with_capacity(backend: B, capacity: usize) -> Result<Self> {
        Ok(Self {
            backend: IndexedFile::with_capacity(backend, capacity)?,
            headers: Vec::with_capacity(capacity),
        })
    }
}

impl<B: Backend> Initiable<B> for MultiFile<B> {
    fn init(backend: B) -> Result<Self> {
        let mut backend = IndexedFile::init(backend)?;
        let mut headers: Vec<BaseHeader> = Vec::with_capacity(backend.count());

        for i in 0..backend.count() {
            let entry = backend.entry(i)?;
            let entry_be = BaseBackend::from_storage(entry)?;
            headers.push(*entry_be.header());
        }

        Ok(Self { backend, headers })
    }
}

impl<B: Backend> MType for MultiFile<B> {
    #[inline]
    fn raw_data(&self) -> &[u8] {
        self.backend.raw_data()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::backend::memory::test::make_mem_backend;
    use crate::backend::mmap_mut::test::make_mmap_backend;
    use crate::components::header_file::CustomHeaderFile;
    use crate::components::indexed_file::entry::test::{
        insert_test_data, TEST_DATA_1, TEST_DATA_2, TEST_DATA_3,
    };
    use crate::traits::creatable::MemCreatable;

    #[test]
    fn test_all() {
        let backend = make_mem_backend(10);
        let mut mfile = MultiFile::create(backend).unwrap();
        test_simple(&mut mfile);
        test(&mut mfile);

        let backend = make_mmap_backend("./mfile", 10);
        let mut mfile = MultiFile::create(backend).unwrap();
        test_simple(&mut mfile);
        test(&mut mfile);
    }

    fn test_simple<B: GrowableBackend>(mfile: &mut MultiFile<B>) {
        let mut ifile = IndexedFile::create_mem_with_capacity(4).unwrap();
        insert_test_data(&mut ifile);

        let mut ifile2 = IndexedFile::create_mem_with_capacity(4).unwrap();
        insert_test_data(&mut ifile2);

        mfile.insert(&ifile).unwrap();
        mfile.insert(&ifile2).unwrap();

        let ifile_loaded = mfile.get(0).unwrap();
        assert_eq!(ifile_loaded.data(), ifile.raw_data());
        let mut if1 = IndexedFile::init(ifile_loaded).unwrap();
        assert_eq!(if1.get(0), Ok(TEST_DATA_1));
        assert_eq!(if1.entry(0).unwrap().data(), TEST_DATA_1);
        assert_eq!(if1.get(1), Ok(TEST_DATA_2));
        assert_eq!(if1.entry(1).unwrap().data(), TEST_DATA_2);
        assert_eq!(if1.get(2), Ok(TEST_DATA_3));
        assert_eq!(if1.entry(2).unwrap().data(), TEST_DATA_3);

        assert_eq!(mfile.count(), 2);
        mfile.clear();

        let mut ofile = mfile.insert_new_backend::<IndexedFile<_>>().unwrap();
        insert_test_data(&mut ofile);
        let ofile_data: Vec<_> = ofile.iter().map(|i| i.to_vec()).collect();

        let mut ofile2 = mfile.insert_new_backend::<IndexedFile<_>>().unwrap();
        insert_test_data(&mut ofile2);
        let ofile2_data: Vec<_> = ofile2.iter().map(|i| i.to_vec()).collect();

        let ifile_loaded = mfile.get(0).unwrap();
        assert_eq!(ifile_loaded.data(), ifile.raw_data());
        let mut if1 = IndexedFile::init(ifile_loaded).unwrap();
        assert_eq!(if1.get(0), Ok(TEST_DATA_1));
        assert_eq!(if1.entry(0).unwrap().data(), TEST_DATA_1);
        assert_eq!(if1.get(1), Ok(TEST_DATA_2));
        assert_eq!(if1.entry(1).unwrap().data(), TEST_DATA_2);
        assert_eq!(if1.get(2), Ok(TEST_DATA_3));
        assert_eq!(if1.entry(2).unwrap().data(), TEST_DATA_3);

        let (ofile, ofile2) = mfile.get_two_mut(0, 1).unwrap();
        let ofile = IndexedFile::init(ofile).unwrap();
        let ofile2 = IndexedFile::init(ofile2).unwrap();
        assert_eq!(ofile.iter().collect::<Vec<_>>(), ofile_data);
        assert_eq!(ofile2.iter().collect::<Vec<_>>(), ofile2_data);
    }

    fn test<B: GrowableBackend>(mfile: &mut MultiFile<B>) {
        let mut sub_mfile = MultiFile::create_mem().unwrap();

        // let mut list1 = IndexedFile::create(make_mem_backend(10)).unwrap();
        let mut list1: IndexedFile<_> = sub_mfile.insert_new_backend().unwrap();
        list1.insert(&[1]).unwrap();
        list1.insert(&[2, 2]).unwrap();
        list1.insert(&[3, 3, 3]).unwrap();
        list1.insert(&[42]).unwrap();
        // sub_mfile.insert(&list1).unwrap();

        let mut list2 = IndexedFile::create_mem().unwrap();
        list2.insert(&[1]).unwrap();
        list2.insert(&[2, 2]).unwrap();
        list2.insert(&[3, 3, 3]).unwrap();
        list2.insert(&[42]).unwrap();
        sub_mfile.insert(&list2).unwrap();

        mfile.insert(&sub_mfile).unwrap();
        mfile.insert(&sub_mfile).unwrap();

        for i in (0..1300).step_by(13) {
            mfile.grow(0, i).unwrap();

            sub_mfile.grow(i % 2, i % 10).unwrap();
            // let mut list2_got = IndexedFile::init(sub_mfile.entry_mut(1).unwrap()).unwrap();
            let mut list2_got: IndexedFile<_> = sub_mfile.get_backend_mut(1).unwrap();

            assert_eq!(list2_got.get(0), Ok(&[1][..]));
            assert_eq!(list2_got.entry(0).unwrap().data(), &[1][..]);
            assert_eq!(list2_got.get(1), Ok(&[2, 2][..]));
            assert_eq!(list2_got.entry(1).unwrap().data(), &[2, 2][..]);
            assert_eq!(list2_got.get(2), Ok(&[3, 3, 3][..]));
            assert_eq!(list2_got.entry(2).unwrap().data(), &[3, 3, 3][..]);

            sub_mfile.grow(i % 2, i % 10).unwrap();
        }

        let mut chf = sub_mfile
            .insert_new_backend::<CustomHeaderFile<_, String>>()
            .unwrap();
        chf.grow(100).unwrap();
        let header_text = "abclolあおさおえぬｈたおせうんｔｈ";
        chf.set_header(header_text.to_string()).unwrap();

        let mut list2_got: IndexedFile<_> = sub_mfile.get_backend_mut(1).unwrap();
        list2_got.insert_t(&"this is a test").unwrap();

        let chf: CustomHeaderFile<_, String> = sub_mfile.get_backend_mut(2).unwrap();
        assert_eq!(chf.header(), header_text);
    }
}

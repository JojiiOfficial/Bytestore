use crate::backend::Backend;
use crate::components::indexed_file::IndexedFile;

pub struct IndexedFileIter<'i, B> {
    ifile: &'i IndexedFile<B>,
    pos: usize,
    pos_end: usize,
}

impl<'i, B> IndexedFileIter<'i, B> {
    #[inline]
    pub(crate) fn new(ifile: &'i IndexedFile<B>) -> Self {
        Self {
            ifile,
            pos: 0,
            pos_end: 0,
        }
    }
}

impl<'i, B> Iterator for IndexedFileIter<'i, B>
where
    B: Backend,
{
    type Item = &'i [u8];

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let item = self.ifile.get(self.pos).ok()?;
        self.pos += 1;
        Some(item)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.ifile.count(), Some(self.ifile.count()))
    }

    #[inline]
    fn last(self) -> Option<Self::Item>
    where
        Self: Sized,
    {
        self.ifile.get(self.ifile.count - 1).ok()
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.pos += n;
        self.ifile.get(self.pos).ok()
    }
}

impl<'i, B> DoubleEndedIterator for IndexedFileIter<'i, B>
where
    B: Backend,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.pos_end >= self.ifile.count {
            return None;
        }
        let index = self.ifile.count - 1 - self.pos_end;
        let item = self.ifile.get(index).ok()?;
        self.pos_end += 1;
        Some(item)
    }
}

impl<'i, B> ExactSizeIterator for IndexedFileIter<'i, B>
where
    B: Backend,
{
    #[inline]
    fn len(&self) -> usize {
        self.ifile.count()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::backend::growable::GrowableBackend;
    use crate::backend::memory::test::make_mem_backend;
    use crate::backend::mmap_mut::test::make_mmap_backend;
    use crate::components::indexed_file::entry::test::{insert_test_data, ALL_TEST_DATA};
    use crate::traits::creatable::Creatable;

    #[test]
    fn test_all() {
        let backend = make_mem_backend(50);
        let mut ifile = IndexedFile::create(backend).unwrap();
        test_iter(&mut ifile);

        let backend = make_mmap_backend("./if_iter", 50);
        let mut ifile = IndexedFile::create(backend).unwrap();
        test_iter(&mut ifile);
    }

    fn test_iter<B: GrowableBackend>(ifile: &mut IndexedFile<B>) {
        ifile.clear();
        insert_test_data(ifile);

        let iter = IndexedFileIter::new(ifile);
        assert_eq!(iter.collect::<Vec<_>>(), ALL_TEST_DATA);

        let iter = IndexedFileIter::new(ifile);
        let mut exp = ALL_TEST_DATA.to_vec();
        exp.reverse();
        assert_eq!(iter.rev().collect::<Vec<_>>(), exp);

        let iter = IndexedFileIter::new(ifile);
        assert_eq!(iter.last(), Some(ALL_TEST_DATA[ALL_TEST_DATA.len() - 1]));
    }
}

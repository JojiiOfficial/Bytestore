use crate::backend::growable::GrowableBackend;
use crate::backend::Backend;
use crate::components::multi_file::MultiFile;
use crate::header::BaseHeader;
use crate::Result;

/// A single entry in a `MultiFile` implementing `GrowableBackend`.
pub struct MFileEntryMut<'a, B> {
    file: &'a mut MultiFile<B>,
    id: usize,
}

impl<'a, B> MFileEntryMut<'a, B> {
    #[inline]
    pub(super) fn new(file: &'a mut MultiFile<B>, id: usize) -> Self {
        Self { file, id }
    }
}

impl<'a, B> Backend for MFileEntryMut<'a, B>
where
    B: Backend,
{
    #[inline]
    fn data(&self) -> &[u8] {
        self.file.get_be_data(self.id).unwrap()
    }

    #[inline]
    fn data_mut(&mut self) -> &mut [u8] {
        self.file.get_be_data_mut(self.id).unwrap()
    }

    #[inline]
    fn first_index(&self) -> usize {
        BaseHeader::len_bytes()
    }

    #[inline]
    fn len(&self) -> usize {
        self.file.get(self.id).unwrap().len()
    }

    #[inline]
    fn set_len(&mut self, len: usize) -> Result<()> {
        self.file.get_mut(self.id).unwrap().set_len(len)
    }
}

impl<'a, B> GrowableBackend for MFileEntryMut<'a, B>
where
    B: GrowableBackend,
{
    fn resize_impl(&mut self, _new_size: usize, _growing: bool) -> Result<()> {
        todo!()
    }

    #[inline]
    fn grow(&mut self, size: usize) -> Result<()> {
        self.file.grow(self.id, size)
    }

    #[inline]
    fn shrink(&mut self, size: usize) -> Result<()> {
        self.file.shrink(self.id, size)
    }

    fn resize(&mut self, delta: isize) -> Result<()> {
        if delta >= 0 {
            self.grow(delta as usize)
        } else {
            self.shrink(delta.unsigned_abs())
        }
    }
}

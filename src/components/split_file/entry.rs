use crate::backend::growable::GrowableBackend;
use crate::backend::Backend;
use crate::components::split_file::backend_index::BackendIndex;
use crate::components::split_file::SplitFile;
use crate::header::BaseHeader;
use crate::Error;

/// A backend in a `SplitFile` (either the first or the second one)
pub struct Entry<'a, B> {
    sfile: &'a mut SplitFile<B>,
    index: BackendIndex,
}

impl<'a, B> Entry<'a, B> {
    #[inline]
    pub(super) fn new(sfile: &'a mut SplitFile<B>, index: BackendIndex) -> Self {
        Self { sfile, index }
    }
}

impl<'a, B> Backend for Entry<'a, B>
    where
        B: Backend,
{
    #[inline]
    fn data(&self) -> &[u8] {
        self.sfile.backend_data(self.index)
    }

    #[inline]
    fn data_mut(&mut self) -> &mut [u8] {
        self.sfile.backend_data_mut(self.index)
    }

    fn first_index(&self) -> usize {
        BaseHeader::len_bytes()
    }

    #[inline]
    fn len(&self) -> usize {
        self.sfile.get_header_for(self.index).data_len()
    }

    #[inline]
    fn set_len(&mut self, len: usize) -> Result<(), Error> {
        self.sfile.get_backend_mut(self.index).set_len(len)
    }

    #[inline]
    fn flush_range_impl(&mut self, start: usize, len: usize) -> Result<(), Error> {
        self.sfile.flush_backend_range(self.index, start, len)
    }
}

impl<'a, B> GrowableBackend for Entry<'a, B>
    where
        B: GrowableBackend,
{
    fn resize_impl(&mut self, _new_size: usize, _: bool) -> crate::Result<()> {
        unreachable!()
    }

    #[inline]
    fn grow(&mut self, size: usize) -> crate::Result<()> {
        self.sfile.grow(self.index, size)
    }

    #[inline]
    fn resize(&mut self, delta: isize) -> crate::Result<()> {
        self.sfile.resize(self.index, delta)
    }
}

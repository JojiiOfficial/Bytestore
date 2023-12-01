use std::ptr::slice_from_raw_parts;
use crate::backend::growable::GrowableBackend;
use crate::backend::Backend;
use crate::traits::mtype::MType;
use crate::Error;

/// A backend that takes the entire underlying storage as backend.
/// This means len() == capacity() is always true and set_len will return an error.
#[derive(Copy, Clone)]
pub struct FullBackend<S> {
    storage: S,
}

impl<S> FullBackend<S> {
    #[inline]
    pub fn new(storage: S) -> Self {
        FullBackend { storage }
    }
}

impl FullBackend<&[u8]> {
    /// Retruns a FullBackend that doesn't depend on lifetimes (The returning backend IS NOT 'static!).
    ///
    /// # Safety
    /// The user of the backend returned by this function has to ensure that they only use it as long as the original
    /// backend data is valid.
    #[inline]
    pub unsafe fn ignore_lifetimes(self) -> FullBackend<&'static [u8]> {
        let len = self.storage.len();
        let ptr = self.storage.as_ptr();
        let data = &*slice_from_raw_parts(ptr, len);
        FullBackend::new(data)
    }
}

impl<S> MType for FullBackend<S>
    where
        Self: Backend,
{
    #[inline]
    fn raw_data(&self) -> &[u8] {
        self.data()
    }
}

impl<B> Backend for FullBackend<B>
    where
        B: Backend,
{
    #[inline]
    fn data(&self) -> &[u8] {
        self.storage.data()
    }
    #[inline]
    fn data_mut(&mut self) -> &mut [u8] {
        self.storage.data_mut()
    }

    #[inline]
    fn first_index(&self) -> usize {
        self.storage.first_index()
    }

    #[inline]
    fn len(&self) -> usize {
        self.storage.capacity()
    }

    #[inline]
    fn set_len(&mut self, len: usize) -> Result<(), Error> {
        self.storage.set_len(len)
    }
}

impl<S> GrowableBackend for FullBackend<S>
    where
        S: GrowableBackend,
{
    #[inline]
    fn resize_impl(&mut self, _new_size: usize, _growing: bool) -> crate::Result<()> {
        unreachable!()
    }

    #[inline]
    fn grow(&mut self, size: usize) -> crate::Result<()> {
        self.storage.grow(size)
    }

    #[inline]
    fn shrink(&mut self, size: usize) -> crate::Result<()> {
        self.storage.shrink(size)
    }

    #[inline]
    fn resize(&mut self, delta: isize) -> crate::Result<()> {
        self.storage.resize(delta)
    }
}

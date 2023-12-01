pub mod sub;
pub mod sub_mut;

use crate::backend::base::sub::BaseSubBackend;
use crate::backend::base::sub_mut::BaseSubMutBackend;
use crate::backend::Backend;
use crate::error::Error;
use crate::header::BaseHeader;
use crate::traits::creatable::Creatable;
use crate::traits::mtype::MType;
use std::ops::{Deref, DerefMut, Range};

pub struct BaseBackend<S> {
    storage: S,
    header: BaseHeader,
}

impl<S> BaseBackend<S> {
    #[inline]
    pub fn storage(&self) -> &S {
        &self.storage
    }

    #[inline]
    pub(crate) fn storage_mut(&mut self) -> &mut S {
        &mut self.storage
    }

    #[inline]
    pub fn header(&self) -> &BaseHeader {
        &self.header
    }

    #[inline]
    pub fn header_len() -> usize {
        BaseHeader::len_bytes()
    }
}

impl<S> Creatable<S> for BaseBackend<S>
    where
        S: Deref<Target=[u8]>,
{
    fn with_capacity(backend: S, _: usize) -> crate::Result<Self> {
        let header_bytes: [u8; 8] = (&backend.deref()[0..8])
            .try_into()
            .map_err(|_| Error::InvalidHeader)?;
        let header = BaseHeader::from_bytes(header_bytes);
        Ok(BaseBackend {
            storage: backend,
            header,
        })
    }
}

impl<S> BaseBackend<S>
    where
        S: Deref<Target=[u8]>,
{
    /// Loads a ByteBackend from a storage that already contains a header (and optionally data).
    pub fn from_storage(storage: S) -> Result<Self, Error> {
        let header_bytes: [u8; 8] = (&storage.deref()[0..8])
            .try_into()
            .map_err(|_| Error::InvalidHeader)?;
        let header = BaseHeader::from_bytes(header_bytes);
        Ok(Self { storage, header })
    }
}

impl<S> BaseBackend<S>
    where
        S: DerefMut<Target=[u8]>,
{
    pub(crate) fn set_len(&mut self, len: usize) -> Result<(), Error> {
        self.header.set_data_len(len);
        self.write_header(self.header);
        Ok(())
    }

    #[inline]
    pub(crate) fn header_range() -> Range<usize> {
        0..BaseHeader::len_bytes()
    }

    #[inline]
    fn write_header(&mut self, header: BaseHeader) {
        self.storage.deref_mut()[Self::header_range()].copy_from_slice(&header.bytes());
    }
}

impl<S> BaseBackend<S>
    where
        Self: Backend,
        S: DerefMut<Target=[u8]>,
{
    pub fn sub_backend(&self, capacity: usize) -> Result<BaseSubBackend<&[u8]>, Error> {
        let end = capacity + Self::header_len();
        self.check_capacity_oob(end)?;
        let storage = &self.storage[..end];
        Ok(BaseSubBackend::new(storage, &self.header))
    }

    pub fn sub_backend_mut(
        &mut self,
        capacity: usize,
    ) -> Result<BaseSubMutBackend<&mut [u8]>, Error> {
        let end = capacity + Self::header_len();
        self.check_capacity_oob(end)?;
        let storage = &mut self.storage[..end];
        Ok(BaseSubMutBackend::new(storage, &mut self.header))
    }
}

impl<S> MType for BaseBackend<S>
    where
        Self: Backend,
{
    #[inline]
    fn raw_data(&self) -> &[u8] {
        self.data()
    }
}

impl Backend for BaseBackend<&mut [u8]> {
    #[inline]
    fn data(&self) -> &[u8] {
        self.storage
    }

    #[inline]
    fn data_mut(&mut self) -> &mut [u8] {
        self.storage.deref_mut()
    }

    #[inline]
    fn first_index(&self) -> usize {
        Self::header_len()
    }

    #[inline]
    fn len(&self) -> usize {
        self.header.data_len()
    }

    fn set_len(&mut self, len: usize) -> Result<(), Error> {
        self.check_capacity_oob(len)?;
        self.header.set_data_len(len);
        self.write_header(self.header);
        Ok(())
    }
}

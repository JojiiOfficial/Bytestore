use crate::backend::Backend;
use crate::error::Error;
use crate::header::BaseHeader;
use crate::traits::mtype::MType;
use std::ops::DerefMut;

pub type GeneralSubMutBackend<'a> = BaseSubMutBackend<'a, &'a mut [u8]>;

pub struct BaseSubMutBackend<'a, S> {
    storage: S,
    header: &'a mut BaseHeader,
}

impl<'a, S> BaseSubMutBackend<'a, S> {
    pub fn new(storage: S, header: &'a mut BaseHeader) -> Self {
        Self { storage, header }
    }
}

impl<'a, S> BaseSubMutBackend<'a, S>
where
    S: DerefMut<Target = [u8]>,
{
    fn write_header(&mut self, header: BaseHeader) {
        let len = BaseHeader::len_bytes();
        self.storage.deref_mut()[0..len].copy_from_slice(&header.bytes());
    }
}

impl<'a> MType for BaseSubMutBackend<'a, &mut [u8]> {
    #[inline]
    fn raw_data(&self) -> &[u8] {
        self.data()
    }
}

impl<'a> Backend for BaseSubMutBackend<'a, &mut [u8]> {
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
        BaseHeader::len_bytes()
    }

    #[inline]
    fn len(&self) -> usize {
        self.header.data_len()
    }

    fn set_len(&mut self, len: usize) -> Result<(), Error> {
        self.check_capacity_oob(len)?;
        self.header.set_data_len(len);
        self.write_header(*self.header);
        Ok(())
    }
}

use crate::backend::Backend;
use crate::error::Error;
use crate::header::BaseHeader;
use crate::traits::mtype::MType;
use std::ops::DerefMut;

pub type GeneralSubBackend<'a> = BaseSubBackend<'a, &'a [u8]>;

pub struct BaseSubBackend<'a, S> {
    storage: S,
    header: &'a BaseHeader,
}

impl<'a, S> BaseSubBackend<'a, S> {
    pub fn new(storage: S, header: &'a BaseHeader) -> Self {
        Self { storage, header }
    }
}

impl<'a, S> BaseSubBackend<'a, S>
where
    S: DerefMut<Target = [u8]>,
{
    //
}

impl<'a> MType for BaseSubBackend<'a, &[u8]> {
    #[inline]
    fn raw_data(&self) -> &[u8] {
        self.data()
    }
}

impl<'a> Backend for BaseSubBackend<'a, &[u8]> {
    #[inline]
    fn data(&self) -> &[u8] {
        self.storage
    }

    #[inline]
    fn data_mut(&mut self) -> &mut [u8] {
        panic!("Backend is read only!")
    }

    #[inline]
    fn first_index(&self) -> usize {
        BaseHeader::len_bytes()
    }

    #[inline]
    fn len(&self) -> usize {
        self.header.data_len()
    }

    fn set_len(&mut self, _: usize) -> Result<(), Error> {
        panic!("Backend is read only!")
    }
}

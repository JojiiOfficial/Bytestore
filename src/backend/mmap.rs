use crate::backend::base::BaseBackend;
use crate::backend::Backend;
use crate::error::Error;
use crate::header::BaseHeader;
use memmap2::{Mmap, MmapOptions};
use std::fs::{File, OpenOptions};
use std::ops::{Deref};
use std::path::Path;

pub type MmapBackend = BaseBackend<MmapFile>;

pub struct MmapFile {
    map: Mmap,
}

impl MmapFile {
    /// Loads an existing mmap file from a path read only.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        fn inner(path: &Path) -> Result<MmapFile, Error> {
            let file = OpenOptions::new()
                .read(true)
                .write(false)
                .create(false)
                .open(path)?;
            MmapFile::from_file(file)
        }

        inner(path.as_ref())
    }

    /// Loads a MapStore directly from a `File`.
    pub fn from_file(file: File) -> crate::Result<Self> {
        let map = unsafe { MmapOptions::new().map(&file)? };
        Ok(MmapFile { map })
    }
}

impl Backend for MmapBackend {
    #[inline]
    fn data(&self) -> &[u8] {
        self.storage().deref()
    }

    #[inline]
    fn data_mut(&mut self) -> &mut [u8] {
        panic!("Read only backend");
    }

    #[inline]
    fn first_index(&self) -> usize {
        BaseHeader::len_bytes()
    }

    #[inline]
    fn len(&self) -> usize {
        self.header().data_len()
    }

    #[inline]
    fn set_len(&mut self, _len: usize) -> Result<(), Error> {
        panic!("Read only backend");
    }

    #[inline]
    fn flush_range_impl(&mut self, _: usize, _: usize) -> Result<(), Error> {
        Ok(())
    }
}

impl Deref for MmapFile {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.map.deref()
    }
}

#[cfg(test)]
pub mod test {
    // use super::*;

    // TODO!
}

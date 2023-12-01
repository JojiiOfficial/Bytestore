use crate::backend::base::BaseBackend;
use crate::backend::growable::GrowableBackend;
use crate::backend::Backend;
use crate::error::Error;
use crate::header::BaseHeader;
use memmap2::{MmapMut, MmapOptions, RemapOptions};
use std::fs::{File, OpenOptions};
use std::ops::{Deref, DerefMut};
use std::path::Path;

pub type MmapBackendMut = BaseBackend<MmapFileMut>;

pub struct MmapFileMut {
    file: File,
    map: MmapMut,
}

impl MmapFileMut {
    /// Creates a new file and truncase an existing one.
    pub fn create<P: AsRef<Path>>(path: P, size: usize) -> Result<Self, Error> {
        fn inner(path: &Path, size: usize) -> Result<MmapFileMut, Error> {
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(path)?;

            let file_size = size + BaseHeader::len_bytes();
            file.set_len(file_size as u64)?;

            MmapFileMut::from_file(file)
        }

        inner(path.as_ref(), size)
    }

    /// Loads an existing mmap file from a path read only.
    pub fn load_ro<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        fn inner(path: &Path) -> Result<MmapFileMut, Error> {
            let file = OpenOptions::new()
                .read(true)
                .write(false)
                .create(false)
                .open(path)?;
            MmapFileMut::from_file(file)
        }

        inner(path.as_ref())
    }

    /// Loads an existing mmap file from a path.
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        fn inner(path: &Path) -> Result<MmapFileMut, Error> {
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(false)
                .open(path)?;
            MmapFileMut::from_file(file)
        }

        inner(path.as_ref())
    }

    /// Loads a MapStore directly from a `File`.
    pub fn from_file(file: File) -> crate::Result<Self> {
        let map = unsafe { MmapOptions::new().map_mut(&file)? };
        Ok(MmapFileMut { file, map })
    }

    #[inline]
    pub fn flush_range(&mut self, start: usize, len: usize) -> crate::Result<()> {
        self.map.flush_range(start, len)?;
        Ok(())
    }
}

impl Backend for MmapBackendMut {
    #[inline]
    fn data(&self) -> &[u8] {
        self.storage().deref()
    }

    #[inline]
    fn data_mut(&mut self) -> &mut [u8] {
        self.storage_mut().deref_mut()
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
    fn set_len(&mut self, len: usize) -> Result<(), Error> {
        self.set_len(len)?;
        Ok(())
    }

    #[inline]
    fn flush_range_impl(&mut self, start: usize, len: usize) -> Result<(), Error> {
        self.storage_mut().flush_range(start, len)
    }
}

impl GrowableBackend for MmapBackendMut {
    fn resize_impl(&mut self, new_len: usize, _: bool) -> crate::Result<()> {
        // println!("Growing: {new_len}");
        self.storage_mut().file.set_len(new_len as u64).unwrap();
        unsafe {
            self.storage_mut()
                .map
                .remap(new_len, RemapOptions::new().may_move(true))
                .unwrap();
        }

        Ok(())
    }
}

impl Deref for MmapFileMut {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.map.deref()
    }
}

impl DerefMut for MmapFileMut {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.map.deref_mut()
    }
}

#[cfg(test)]
pub mod test {
    use super::super::test::*;
    use super::*;

    pub fn make_mmap_backend(file: &str, len: usize) -> MmapBackendMut {
        let path = Path::new("./testfiles");

        if !path.is_dir() {
            std::fs::create_dir_all(path).unwrap();
        }

        let path = path.join(file);
        MmapBackendMut::from_storage(MmapFileMut::create(path, len).unwrap()).unwrap()
    }

    #[test]
    fn test_load() {
        let mut file_backend = make_mmap_backend("./plsloadme", 100);
        file_backend.push(&[10, 10, 9, 123]).unwrap();
        drop(file_backend);

        let loaded_backend =
            MmapBackendMut::from_storage(MmapFileMut::load("./testfiles/plsloadme").unwrap())
                .unwrap();
        assert_eq!(loaded_backend.get(0, 4), Ok(&[10, 10, 9, 123][..]));
    }

    #[test]
    fn mmap_backend() {
        let small_backend = make_mmap_backend("./stest", 100);
        test_mmap_backend_all(small_backend);

        let big_backend = make_mmap_backend("./stest_big", 1024 * 1024);
        test_mmap_backend_all(big_backend);
    }

    fn test_mmap_backend_all<S: DerefMut<Target=[u8]>>(mut backend: BaseBackend<S>)
        where
            BaseBackend<S>: Backend,
    {
        assert!(backend.is_empty());
        be_clear(&mut backend);
        be_push(&mut backend);
        be_replace(&mut backend);
        be_replace(&mut backend);
        be_fill(&mut backend);

        if backend.capacity() > 61 * 1024 {
            be_stresstest(&mut backend);
        }
    }
}

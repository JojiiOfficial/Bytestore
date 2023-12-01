use crate::backend::growable::GrowableBackend;
use crate::backend::Backend;
use crate::error::Error;
use crate::traits::creatable::Creatable;
use crate::traits::deser::Deser;
use crate::traits::initiable::Initiable;
use crate::traits::mtype::MType;

pub struct CustomHeaderFile<B, H> {
    pub(crate) backend: B,
    header: H,
    header_len: u32,
}

impl<B, H> Initiable<B> for CustomHeaderFile<B, H>
where
    B: Backend,
    H: Deser,
{
    fn init(backend: B) -> crate::Result<Self> {
        let header_len: u32 = backend.get_t(0, 4).map_err(|_| Error::InvalidHeader)?;

        let header_data = backend
            .get(4, header_len as usize - 4)
            .map_err(|_| Error::InvalidHeader)?;

        let header = bitcode::deserialize(header_data)?;
        Ok(Self {
            backend,
            header,
            header_len,
        })
    }
}

impl<B, H> Creatable<B> for CustomHeaderFile<B, H>
where
    B: GrowableBackend,
    H: Deser + Default,
{
    fn with_capacity(mut backend: B, _: usize) -> crate::Result<Self> {
        let header = H::default();
        let header_data = bitcode::serialize(&header)?;
        let header_len = header_data.len() as u32 + 4;

        if backend.capacity() < header_len as usize {
            backend.grow(header_len as usize)?;
        }

        let b = backend.push_t(&header_len)?;
        assert_eq!(b.1, 4);

        backend.push(&header_data)?;

        Ok(Self {
            backend,
            header,
            header_len,
        })
    }
}

impl<B, H> CustomHeaderFile<B, H>
where
    B: Backend,
    H: Deser,
{
    pub fn create(mut backend: B, header: H) -> Result<Self, Error> {
        let header_data = bitcode::serialize(&header)?;
        let header_len = header_data.len() as u32 + 4;

        let b = backend.push_t(&header_len)?;
        assert_eq!(b.1, 4);

        backend.push(&header_data)?;

        backend.flush()?;

        Ok(Self {
            backend,
            header,
            header_len,
        })
    }

    pub fn set_header(&mut self, new_header: H) -> Result<(), Error> {
        let new_header_len =
            self.backend
                .replace_t(4, self.header_len as usize - 4, &new_header)? as u32
                + 4;
        self.backend.replace_same_len_t(0, &new_header_len)?;
        self.backend.flush_range(0, self.header_len as usize)?;
        self.header_len = new_header_len;
        self.header = new_header;
        Ok(())
    }

    #[inline]
    pub fn backend(&self) -> &B {
        &self.backend
    }
}

impl<B, H> MType for CustomHeaderFile<B, H>
where
    B: Backend,
{
    #[inline]
    fn raw_data(&self) -> &[u8] {
        self.data()
    }
}

impl<B, H> CustomHeaderFile<B, H> {
    #[inline]
    pub fn header_len(&self) -> usize {
        self.header_len as usize
    }

    #[inline]
    pub fn header(&self) -> &H {
        &self.header
    }
}

impl<B: Backend, H> Backend for CustomHeaderFile<B, H> {
    #[inline]
    fn data(&self) -> &[u8] {
        self.backend.data()
    }

    #[inline]
    fn data_mut(&mut self) -> &mut [u8] {
        self.backend.data_mut()
    }

    #[inline]
    fn first_index(&self) -> usize {
        self.backend.first_index() + self.header_len as usize
    }

    #[inline]
    fn len(&self) -> usize {
        self.backend.len().saturating_sub(self.header_len())
    }

    #[inline]
    fn set_len(&mut self, len: usize) -> Result<(), Error> {
        self.backend.set_len(len + self.header_len())
    }

    #[inline]
    fn flush_range_impl(&mut self, start: usize, len: usize) -> Result<(), Error> {
        let hl = self.header_len as usize;
        self.backend.flush_range(start + hl, len - hl)
    }
}

impl<B, H> GrowableBackend for CustomHeaderFile<B, H>
where
    B: GrowableBackend,
{
    fn resize_impl(&mut self, _new_size: usize, _growing: bool) -> crate::Result<()> {
        unreachable!()
    }

    #[inline]
    fn grow(&mut self, size: usize) -> crate::Result<()> {
        self.backend.grow(size)
    }

    #[inline]
    fn shrink(&mut self, size: usize) -> crate::Result<()> {
        self.backend.shrink(size)
    }

    #[inline]
    fn resize(&mut self, delta: isize) -> crate::Result<()> {
        self.backend.resize(delta)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::backend::base::BaseBackend;
    use crate::backend::memory::test::make_mem_backend;
    use crate::backend::mmap_mut::test::make_mmap_backend;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
    struct TestHeader {
        s: String,
        n: usize,
        o: u128,
    }

    impl TestHeader {
        pub fn new(s: String, n: usize, o: u128) -> Self {
            Self { s, n, o }
        }
    }

    fn custom_header<B: Backend>(mut mem_backend: &mut B) {
        let header = TestHeader::new("lol".to_string(), 125, 1290876519283);
        let header_bytes = bitcode::serialize(&header).unwrap();

        let mut ch_file = CustomHeaderFile::create(&mut mem_backend, header.clone()).unwrap();

        let h_len = ch_file.header_len();

        assert_eq!(ch_file.len(), 0);
        ch_file.push(&[53]).unwrap();
        assert_eq!(ch_file.len(), 1);

        drop(ch_file);

        // First four bytes are length of header
        assert_eq!(
            mem_backend.get_t::<u32>(0, 4),
            Ok(header_bytes.len() as u32 + 4)
        );
        // 4..h_len are header bytes
        assert_eq!(mem_backend.get(4, h_len - 4), Ok(header_bytes.as_slice()));
        // h_len+4 is our pushed value
        assert_eq!(mem_backend.get(h_len, 1), Ok(&[53][..]));

        let mut ch_file: CustomHeaderFile<_, TestHeader> =
            CustomHeaderFile::init(&mut mem_backend).unwrap();

        assert_eq!(ch_file.header, header);
        assert_eq!(ch_file.get(0, 1), Ok(&[53][..]));
        ch_file.push(&[99]).unwrap();

        ch_file.clear();

        drop(ch_file);

        assert_eq!(mem_backend.get(h_len + 1, 1), Err(Error::OutOfBounds));

        let mut ch_file: CustomHeaderFile<_, TestHeader> =
            CustomHeaderFile::init(&mut mem_backend).unwrap();

        ch_file.push(&[53, 99]).unwrap();
        drop(ch_file);

        assert_eq!(mem_backend.get(h_len + 1, 1), Ok(&[99][..]));

        let mut ch_file: CustomHeaderFile<_, TestHeader> =
            CustomHeaderFile::init(&mut mem_backend).unwrap();

        assert_eq!(ch_file.get(0, 2), Ok(&[53, 99][..]));
        assert_eq!(ch_file.len(), 2);

        let new_header = TestHeader::new("loabscauntsoaheusoaetnhul".to_string(), 115, 12908519283);
        ch_file.set_header(new_header.clone()).unwrap();
        assert_eq!(ch_file.len(), 2);
        assert_eq!(ch_file.get(0, 2), Ok(&[53, 99][..]));

        drop(ch_file);

        let ch_file: CustomHeaderFile<_, TestHeader> =
            CustomHeaderFile::init(&mut mem_backend).unwrap();

        assert_eq!(ch_file.header, new_header);
    }

    fn ch_as_base<B: Backend>(backend: &mut B) {
        backend.clear();

        let data = (10..)
            .step_by(13)
            .map(|i| format!("{i}ao-u{i}-_").repeat(i % 10));
        let mut pushed = vec![];

        for i in data.take(100) {
            pushed.push(i.clone());
            backend.push(i.as_bytes()).unwrap();
        }

        let mut pos = 0;
        for i in pushed.iter() {
            let len = i.as_bytes().len();
            assert_eq!(backend.get(pos, len), Ok(i.as_bytes()));
            pos += len;
        }
    }

    #[test]
    fn test() {
        let mut mem_backend = make_mem_backend(1024 * 1024);
        test_all(&mut mem_backend);

        let mut mmap_backend = make_mmap_backend("./custom_header", 1024 * 1024);
        test_all(&mut mmap_backend);
    }

    fn test_all<S>(backend: &mut BaseBackend<S>)
    where
        BaseBackend<S>: Backend,
    {
        backend.clear();
        let header = TestHeader::new("lol".to_string(), 125, 1290876519283);
        let mut backend = CustomHeaderFile::create(backend, header).unwrap();

        custom_header(&mut backend);
        ch_as_base(&mut backend);
    }
}

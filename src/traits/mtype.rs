pub trait MType {
    fn raw_data(&self) -> &[u8];
}

impl<T> MType for &T
where
    T: MType,
{
    #[inline]
    fn raw_data(&self) -> &[u8] {
        (*self).raw_data()
    }
}

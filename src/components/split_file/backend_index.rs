#[derive(Copy, Clone, PartialOrd, PartialEq, Debug)]
#[repr(usize)]
pub enum BackendIndex {
    First = 1,
    Second = 2,
}

impl BackendIndex {
    /// Returns `true` if the given BackendIndex represents the first index.
    #[inline]
    pub fn is_first(&self) -> bool {
        matches!(self, BackendIndex::First)
    }

    /// Returns `true` if the given BackendIndex represents the second index.
    #[inline]
    pub fn is_second(&self) -> bool {
        matches!(self, BackendIndex::Second)
    }
}

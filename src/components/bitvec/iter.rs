use crate::backend::Backend;
use crate::components::bitvec::BitVec;

/// Iterator over all items in a `BitVec`.
pub struct BitVecIter<'a, B> {
    bv: &'a BitVec<B>,
    pos: usize,
}

impl<'a, B> BitVecIter<'a, B> {
    #[inline]
    pub(super) fn new(bv: &'a BitVec<B>) -> Self {
        Self { bv, pos: 0 }
    }
}

impl<'a, B> Iterator for BitVecIter<'a, B> where B: Backend {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        // TODO: Maybe use a byte cache here, but check whether reading bits actually decreases performance.
        let item = self.bv.get(self.pos)?;
        self.pos += 1;
        Some(item)
    }
}
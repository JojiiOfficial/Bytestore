use crate::backend::Backend;
use crate::components::number_seq::NumberSequence;
use crate::traits::sized_deser::SizedDeser;

pub struct NumberSeqIter<'a, B, T, const N: usize> {
    seq: &'a NumberSequence<B, T, N>,
    pos: usize,
}

impl<'a, B, T, const N: usize> NumberSeqIter<'a, B, T, N> {
    #[inline]
    pub(super) fn new(seq: &'a NumberSequence<B, T, N>) -> Self {
        Self { seq, pos: 0 }
    }
}

impl<'a, B, T, const N: usize> Iterator for NumberSeqIter<'a, B, T, N>
where
    B: Backend,
    T: SizedDeser<N>,
{
    type Item = T;

    // TODO: Implement other functions when needed.

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let item = self.seq.get(self.pos).ok()?;
        self.pos += 1;
        Some(item)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.seq.len();
        (len, Some(len))
    }
}

pub struct OwnedNumberSeqIterator<B, T, const N: usize> {
    seq: NumberSequence<B, T, N>,
    pos: usize,
}

impl<'a, B, T, const N: usize> OwnedNumberSeqIterator<B, T, N> {
    #[inline]
    pub(super) fn new(seq: NumberSequence<B, T, N>) -> Self {
        Self { seq, pos: 0 }
    }
}

impl<B, T, const N: usize> Iterator for OwnedNumberSeqIterator<B, T, N>
where
    B: Backend,
    T: SizedDeser<N>,
{
    type Item = T;

    // TODO: Implement other functions when needed.

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let item = self.seq.get(self.pos).ok()?;
        self.pos += 1;
        Some(item)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.seq.len();
        (len, Some(len))
    }
}

use crate::backend::Backend;
use crate::components::number_seq::compressed::CompressedNumberSequence;
use std::marker::PhantomData;
use varint_simd::VarIntTarget;

pub struct CompressedNumSeqIter<'a, T> {
    slice: &'a [u8],
    start_index: usize,
    len: Option<usize>,
    p: PhantomData<T>,
}

impl<'a, T> CompressedNumSeqIter<'a, T>
where
    T: VarIntTarget,
{
    #[inline]
    pub fn new<B>(num_seq: &'a CompressedNumberSequence<B, T>) -> Self
    where
        B: Backend,
    {
        let slice = num_seq.backend.content_data();
        let len = num_seq.len_opt();
        Self {
            slice,
            start_index: 0,
            len,
            p: PhantomData,
        }
    }

    /// Returns the remaining slice if not empty.
    #[inline]
    fn remaining_slice(&self) -> Option<&'a [u8]> {
        if self.start_index >= self.slice.len() {
            return None;
        }
        Some(&self.slice[self.start_index..])
    }
}

impl<'a, T> Iterator for CompressedNumSeqIter<'a, T>
where
    T: VarIntTarget,
{
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let slice = self.remaining_slice()?;
        let (num, len) = varint_simd::decode(slice).expect("Decoding failed");
        self.start_index += len;
        Some(num)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        match self.len {
            Some(len) => (len, Some(len)),
            None => {
                let lower = self.slice.len();
                let upper = lower * 10;
                (lower, Some(upper))
            }
        }
    }
}

pub struct OwnedCompressedNumSeqIterator<B, T> {
    num_seq: CompressedNumberSequence<B, T>,
    start_index: usize,
}

impl<B, T> OwnedCompressedNumSeqIterator<B, T>
where
    T: VarIntTarget,
{
    #[inline]
    pub fn new(num_seq: CompressedNumberSequence<B, T>) -> Self {
        Self {
            num_seq,
            start_index: 0,
        }
    }
}

impl<B, T> OwnedCompressedNumSeqIterator<B, T>
where
    B: Backend,
    T: VarIntTarget,
{
    /// Returns the remaining slice if not empty.
    #[inline]
    fn remaining_slice(&self) -> Option<&[u8]> {
        let slice = self.num_seq.backend.content_data();
        if self.start_index >= slice.len() {
            return None;
        }
        Some(&slice[self.start_index..])
    }
}

impl<B, T> Iterator for OwnedCompressedNumSeqIterator<B, T>
where
    B: Backend,
    T: VarIntTarget,
{
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let slice = self.remaining_slice()?;
        let (num, len) = varint_simd::decode(slice).expect("Decoding failed");
        self.start_index += len;
        Some(num)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        match self.num_seq.len_opt() {
            Some(len) => (len, Some(len)),
            None => {
                let lower = self.num_seq.backend.content_data().len();
                let upper = lower * 10;
                (lower, Some(upper))
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::components::number_seq::compressed::CompressedNumberSequence;
    use crate::traits::creatable::MemCreatable;
    use std::collections::HashSet;

    const TEST_LEN: usize = 10000;

    #[test]
    fn iter() {
        let mut cns: CompressedNumberSequence<_, u64> =
            CompressedNumberSequence::create_mem_with_capacity(10).unwrap();
        let rand_data = (0..crate::components::number_seq::compressed::iter::test::TEST_LEN)
            .collect::<HashSet<_>>()
            .into_iter()
            .map(|i| i as u64)
            .collect::<Vec<_>>();
        cns.extend(rand_data.iter().copied());

        for (pos, i) in rand_data.iter().enumerate() {
            assert_eq!(cns.get(pos), Some(*i));
        }
        assert_eq!(cns.len(), TEST_LEN);

        for (pos, got) in cns.iter().enumerate() {
            assert_eq!(rand_data[pos], got);
        }
    }
}

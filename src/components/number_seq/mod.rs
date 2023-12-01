pub mod compressed;
pub mod iter;

use crate::backend::growable::GrowableBackend;
use crate::backend::Backend;
use crate::components::number_seq::iter::{NumberSeqIter, OwnedNumberSeqIterator};
use crate::traits::collection::{Collection, GrowableCollection};
use crate::traits::creatable::Creatable;
use crate::traits::initiable::Initiable;
use crate::traits::sized_deser::SizedDeser;
use crate::Error;
use crate::Result;
use std::marker::PhantomData;
use std::slice;

/// Interprets the underlying backend as a list of T which is sized deser.
pub struct NumberSequence<B, T, const N: usize> {
    backend: B,
    p: PhantomData<T>,
}

impl<B, T, const N: usize> NumberSequence<B, T, N> {
    #[inline]
    fn byte_index(index: usize) -> usize {
        index * N
    }
}

impl<B, T, const N: usize> NumberSequence<B, T, N>
    where
        B: Backend,
{
    #[inline]
    pub fn len(&self) -> usize {
        self.backend.len() / N
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.backend.capacity() / N
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.backend.is_empty()
    }

    /// Checks the bounds of `index` and returns an Error if the index is not within the bounds of the NumberSequence.
    #[inline]
    fn check_bounds(&self, index: usize) -> Result<()> {
        if index >= self.len() {
            return Err(Error::OutOfBounds);
        }
        Ok(())
    }
}

impl<B, T, const N: usize> NumberSequence<B, T, N>
    where
        B: Backend,
        T: SizedDeser<N>,
{
    #[inline]
    pub fn set(&mut self, index: usize, val: T) -> Result<()> {
        self.check_bounds(index)?;
        self.backend
            .replace_same_len(Self::byte_index(index), &val.to_bytes())?;
        Ok(())
    }

    #[inline]
    pub fn get(&self, index: usize) -> Result<T> {
        Ok(T::from_bytes(self.get_raw(index)?))
    }

    #[inline]
    pub fn iter(&self) -> NumberSeqIter<B, T, N> {
        NumberSeqIter::new(self)
    }


    #[inline]
    fn get_raw(&self, index: usize) -> Result<[u8; N]> {
        let bindex = Self::byte_index(index);
        let raw: [u8; N] = self.backend.get(bindex, N)?.try_into().unwrap();
        Ok(raw)
    }

    #[inline(always)]
    fn get_raw_unchecked(&self, index: usize) -> [u8; N] {
        let idx = self.backend.get_index(Self::byte_index(index));
        (&self.backend.data()[idx..idx + N]).try_into().unwrap()
    }
}


impl<B, T, const N: usize> NumberSequence<B, T, N>
    where
        B: Backend,
        T: SizedDeser<N> + Ord,
{
    pub fn sort(&mut self) {
        let len = self.len();
        let raw_data = self.backend.content_data_mut();
        assert_eq!(raw_data.len() % N, 0);

        // Safety:
        // We asserted that `raw_data`s len is divisible by N.
        let slices: &mut [[u8; N]] = unsafe { slice::from_raw_parts_mut(raw_data.as_mut_ptr().cast(), len) };
        slices.sort_by(|a, b| {
            let a = T::from_bytes(*a);
            let b = T::from_bytes(*b);
            a.cmp(&b)
        })
    }

    pub fn sort_unstable(&mut self) {
        let len = self.len();
        let raw_data = self.backend.content_data_mut();
        assert_eq!(raw_data.len() % N, 0);

        // Safety:
        // We asserted that `raw_data`s len is divisible by N.
        let slices: &mut [[u8; N]] = unsafe { slice::from_raw_parts_mut(raw_data.as_mut_ptr().cast(), len) };
        slices.sort_unstable_by(|a, b| {
            let a = T::from_bytes(*a);
            let b = T::from_bytes(*b);
            a.cmp(&b)
        })
    }
}

impl<B, T, const N: usize> NumberSequence<B, T, N>
    where
        B: GrowableBackend,
        T: SizedDeser<N>,
{
    /// Grows and appends the given items.
    pub fn append(&mut self, items: &[T]) -> Result<()> {
        let old_len = self.backend.len();
        assert_eq!(old_len % N, 0);

        let cap_inc = items.len() * N;
        self.backend.grow(cap_inc)?;
        let _ = self.backend.set_len(self.backend.len() + cap_inc);

        let mut start_index = old_len;
        for item in items {
            let new_val = item.to_bytes();
            self.backend.replace_same_len(start_index, &new_val)?;
            start_index += N;
        }
        Ok(())
    }

    /// Returns `true` if the numbers are sorted.
    pub fn is_sorted(&self) -> bool {
        if self.len() <= 1 {
            return true;
        }

        for i in 0..self.len() - 1 {
            let this = self.get_raw_unchecked(i);
            let next = self.get_raw_unchecked(i + 1);
            if this > next {
                return false;
            }
        }

        true
    }

    /// Swaps two elements by their index.
    #[inline]
    pub fn swap(&mut self, a: usize, b: usize) -> Result<()> {
        if a == b {
            return Ok(());
        }

        self.check_bounds(a)?;
        self.check_bounds(b)?;
        self.swap_unchecked(a, b);
        Ok(())
    }

    /// Swaps two elements by their index without bounds chechking.
    #[inline]
    pub fn swap_unchecked(&mut self, mut a: usize, mut b: usize) {
        if a > b {
            std::mem::swap(&mut a, &mut b);
        }

        let a_indx = self.backend.get_index(Self::byte_index(a));
        let b_indx = self.backend.get_index(Self::byte_index(b));
        let backend = self.backend.data_mut();
        let (first, second) = backend.split_at_mut(b_indx);
        first[a_indx..a_indx + N].swap_with_slice(&mut second[0..N]);
    }
}

impl<'a, B, T, const N: usize> Extend<&'a T> for NumberSequence<B, T, N>
    where
        B: GrowableBackend,
        T: SizedDeser<N> + Copy + 'a,
{
    #[inline]
    fn extend<I: IntoIterator<Item=&'a T>>(&mut self, iter: I) {
        self.extend(iter.into_iter().copied())
    }
}

impl<B, T, const N: usize> Extend<T> for NumberSequence<B, T, N>
    where
        B: GrowableBackend,
        T: SizedDeser<N>,
{
    fn extend<I: IntoIterator<Item=T>>(&mut self, iter: I) {
        let old_len = self.backend.len();
        assert_eq!(old_len % N, 0);

        let iter = iter.into_iter();

        let (lower, upper) = iter.size_hint();
        let pregrow = Some(lower) == upper;

        if pregrow {
            let cap_inc = lower * N;
            self.backend.grow(cap_inc).expect("failed to grow");
            let _ = self.backend.set_len(self.backend.len() + cap_inc);
        }

        let mut start_index = old_len;
        for item in iter {
            if !pregrow {
                let cap_inc = N;
                self.backend.grow(cap_inc).expect("failed to grow");
                let _ = self.backend.set_len(self.backend.len() + cap_inc);
            }

            let new_val = item.to_bytes();
            self.backend
                .replace_same_len(start_index, &new_val)
                .expect("Failed extending numseq");
            start_index += N;
        }
    }
}

impl<B, T, const N: usize> Collection<T> for NumberSequence<B, T, N>
    where
        B: Backend,
        T: SizedDeser<N>,
{
    type Iter<'a> = NumberSeqIter<'a, B, T, N>
        where
            Self: 'a;

    type Iterator = OwnedNumberSeqIterator<B, T, N>;

    #[inline]
    fn get(&self, index: usize) -> Result<T> {
        self.get(index)
    }

    #[inline]
    fn iter(&self) -> Self::Iter<'_> {
        self.iter()
    }

    #[inline]
    fn into_iter(self) -> Self::Iterator {
        OwnedNumberSeqIterator::new(self)
    }
}

impl<B, T, const N: usize> GrowableCollection<T> for NumberSequence<B, T, N>
    where
        B: GrowableBackend,
        T: SizedDeser<N>,
{
    #[inline]
    fn push(&mut self, item: T) -> Result<()> {
        self.append(&[item])
    }
}

impl<B, T, const N: usize> Creatable<B> for NumberSequence<B, T, N>
    where
        B: GrowableBackend,
        T: SizedDeser<N>,
{
    fn with_capacity(mut backend: B, capacity: usize) -> Result<Self> {
        if capacity > 0 {
            let real_cap = capacity * N;
            backend.grow_to(real_cap)?;
            backend.fill(0..backend.len(), 0)?;
            // let _ = backend.set_len(real_cap).ok();
        }

        if backend.len() % N != 0 {
            return Err(Error::UnexpectedValue);
        }

        Ok(Self {
            backend,
            p: PhantomData,
        })
    }
}

impl<B, T, const N: usize> Initiable<B> for NumberSequence<B, T, N>
    where
        B: Backend,
        T: SizedDeser<N>,
{
    #[inline]
    fn init(backend: B) -> Result<Self> {
        if backend.len() % N != 0 {
            return Err(Error::Initialization);
        }

        Ok(Self {
            backend,
            p: PhantomData,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::backend::full::FullBackend;
    use crate::backend::memory::test::make_mem_backend;
    use std::collections::HashSet;
    use std::time::Instant;

    #[test]
    fn simple_create() {
        let backend = make_mem_backend(0);
        let mut num_seq: NumberSequence<_, u32, 4> =
            NumberSequence::with_capacity(backend, 1).unwrap();
        assert_eq!(num_seq.backend.capacity(), 4);
        assert_eq!(num_seq.len(), 0);

        assert_eq!(num_seq.set(0, 0), Err(Error::OutOfBounds));

        num_seq.append(&[0]).unwrap();

        assert_eq!(num_seq.set(0, 5), Ok(()));
        assert_eq!(num_seq.get(0), Ok(5));
        assert_eq!(num_seq.len(), 1);

        assert_eq!(num_seq.set(1, 9), Err(Error::OutOfBounds));
        assert_eq!(num_seq.get(1), Err(Error::OutOfBounds));
    }

    #[test]
    fn append() {
        let backend = make_mem_backend(1);
        let mut num_seq: NumberSequence<_, u32, 4> =
            NumberSequence::with_capacity(backend, 1).unwrap();

        num_seq.append(&[9, 8, 7]).unwrap();

        assert_eq!(num_seq.get(0), Ok(9));
        assert_eq!(num_seq.get(1), Ok(8));
        assert_eq!(num_seq.get(2), Ok(7));
        assert_eq!(num_seq.get(3), Err(Error::OutOfBounds));
    }

    #[test]
    fn swap() {
        let backend = make_mem_backend(0);
        let mut num_seq: NumberSequence<_, u32, 4> =
            NumberSequence::with_capacity(backend, 0).unwrap();

        let data = &[u32::MAX - 230, u32::MAX - 261293110, u32::MAX - 9991262];

        num_seq.append(data).unwrap();

        assert_eq!(num_seq.get(0), Ok(data[0]));
        assert_eq!(num_seq.get(1), Ok(data[1]));
        assert_eq!(num_seq.get(2), Ok(data[2]));
        assert_eq!(num_seq.get(3), Err(Error::OutOfBounds));
        assert_eq!(num_seq.get_raw_unchecked(0), data[0].to_bytes());
        assert_eq!(num_seq.get_raw_unchecked(1), data[1].to_bytes());
        assert_eq!(num_seq.get_raw_unchecked(2), data[2].to_bytes());

        num_seq.swap(2, 0).unwrap();
        assert_eq!(num_seq.get(0), Ok(data[2]));
        assert_eq!(num_seq.get(1), Ok(data[1]));
        assert_eq!(num_seq.get(2), Ok(data[0]));
        assert_eq!(num_seq.get(3), Err(Error::OutOfBounds));

        num_seq.swap(2, 0).unwrap();
        assert_eq!(num_seq.get(0), Ok(data[0]));
        assert_eq!(num_seq.get(1), Ok(data[1]));
        assert_eq!(num_seq.get(2), Ok(data[2]));
        assert_eq!(num_seq.get(3), Err(Error::OutOfBounds));

        num_seq.swap(0, 1).unwrap();
        assert_eq!(num_seq.get(0), Ok(data[1]));
        assert_eq!(num_seq.get(1), Ok(data[0]));
        assert_eq!(num_seq.get(2), Ok(data[2]));
        assert_eq!(num_seq.get(3), Err(Error::OutOfBounds));

        num_seq.set(0, 0).unwrap();
        num_seq.set(1, 1).unwrap();
        num_seq.set(2, 2).unwrap();
        assert!(num_seq.is_sorted());
    }

    #[test]
    fn sort() {
        for i in (0..24).step_by(3) {
            let backend = make_mem_backend(0);
            let mut num_seq: NumberSequence<_, u32, 4> =
                NumberSequence::with_capacity(backend, 0).unwrap();

            let len = 2usize.pow(i) as u32;

            let rand_nrs = (0..len)
                .collect::<HashSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();

            num_seq.extend(rand_nrs.iter());

            let mut qs_test = rand_nrs.clone();
            let start = Instant::now();
            qs_test.sort_unstable();
            println!("normal vec took: {:?}", start.elapsed());

            for i in 0..rand_nrs.len() {
                assert_eq!(num_seq.get(i), Ok(rand_nrs[i]));
            }
            assert_eq!(num_seq.get(rand_nrs.len()), Err(Error::OutOfBounds));
            let vec = (0..num_seq.len())
                .map(|i| num_seq.get(i).unwrap())
                .collect::<Vec<_>>();
            assert_eq!(vec, rand_nrs);
            assert_eq!(num_seq.len(), rand_nrs.len());

            let start = Instant::now();
            num_seq.sort_unstable();
            println!("Sorted {} elements in {:?}", num_seq.len(), start.elapsed());
            let vec = (0..num_seq.len())
                .map(|i| num_seq.get(i).unwrap())
                .collect::<Vec<_>>();
            assert_eq!(vec, qs_test);
            assert!(num_seq.is_sorted());
        }
    }

    #[test]
    fn full_backend() {
        let mut storage = vec![0u8; 12];
        let be = FullBackend::new(storage.as_mut_slice());
        let mut list: NumberSequence<_, u32, 4> = NumberSequence::init(be).unwrap();

        assert_eq!(list.len(), 3);
        assert_eq!(list.capacity(), 3);

        assert!(list.iter().any(|i| i == 0));

        list.set(0, 109).unwrap();
        assert_eq!(list.get(0), Ok(109));

        let mut new_storage = vec![];
        new_storage.extend_from_slice(&109.to_bytes());
        new_storage.extend((0..8).map(|_| 0));
        assert_eq!(storage, new_storage);
    }
}
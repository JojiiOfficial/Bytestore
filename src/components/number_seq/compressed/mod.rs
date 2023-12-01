pub mod iter;

use crate::backend::growable::GrowableBackend;
use crate::backend::Backend;
use crate::components::number_seq::compressed::iter::{
    CompressedNumSeqIter, OwnedCompressedNumSeqIterator,
};
use crate::traits::collection::{Collection, GrowableCollection};
use crate::traits::creatable::Creatable;
use crate::traits::initiable::Initiable;
use crate::{Error, Result};
use std::marker::PhantomData;
use varint_simd::VarIntTarget;

/// Similar to `super::NumberSequence` but compresses integerss. This is still quite fast as SIMD instructions are used
/// but is definitely slower as using a NumberSequence and get(x) is O(n) in worst case. Iterating over the values
/// using `iter()` is quite fast O(n) compared to iterating over get(x) which is O(n^2).
///
/// The length is unknown and needs to be computed manually to improve initialization.
pub struct CompressedNumberSequence<B, T> {
    backend: B,
    len: Option<usize>,
    p: PhantomData<T>,
}

impl<B, T> CompressedNumberSequence<B, T> {
    /// Returns the length if already calculated. None if not known.
    #[inline]
    pub fn len_opt(&self) -> Option<usize> {
        self.len
    }

    /// Returns `true` if the length is known.
    #[inline]
    pub fn len_known(&self) -> bool {
        self.len.is_some()
    }

    /// Increases the numbers set length by `delta` if the length is known.
    #[inline]
    fn inc_len(&mut self, delta: usize) {
        if let Some(len) = self.len.as_mut() {
            *len += delta;
        }
    }

    #[inline]
    fn oob_check(index: usize, len: usize) -> Result<()> {
        if index >= len {
            return Err(Error::OutOfBounds);
        }
        Ok(())
    }
}

impl<B, T> CompressedNumberSequence<B, T>
where
    B: Backend,
    T: VarIntTarget,
{
    /// Returns the number at `index` or `None` if the index is out of the number sequences bonuds.
    pub fn get(&self, index: usize) -> Option<T> {
        // If we know the length we can do early bounds check to improve performance for out of bounds get() calls.
        // If we don't know the length we'll notice oob automatically when iterating through the values.
        if let Some(len) = self.len {
            Self::oob_check(index, len).ok()?;
        }

        let mut slice = self.backend.content_data();
        let mut len = 0;
        while !slice.is_empty() {
            if len == index {
                let (val, _) = varint_simd::decode::<T>(&slice).expect("bug");
                return Some(val);
            }

            let varint_width = varint_simd::decode_len::<T>(&slice).expect("unexpected");
            slice = &slice[varint_width..];
            len += 1;
        }

        None
    }

    /// Returns the compressed number sets length. If the length is not precalculated using `init_len` it'll always
    /// count the length, so use with caution if performance is important!
    #[inline]
    pub fn len(&self) -> usize {
        if let Some(len) = self.len {
            return len;
        }
        self.calc_len()
    }

    /// Returns `true` if the number sequence doesn't hold any numbers.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.backend.is_empty()
    }

    /// Returns an iterator over all items in the compressed num sequence. If possible using this iterator should be
    /// preferred over multiple get() calls as this might be faster, especially if higher indices get addressed.
    /// Calling `init_len()` before using the iterator can help improve performance if the iterators size_hint() matters.
    #[inline]
    pub fn iter(&self) -> CompressedNumSeqIter<T> {
        CompressedNumSeqIter::new(self)
    }

    /// Calculates and stores the temporary length in the current val.
    #[inline]
    pub fn init_len(&mut self) {
        if self.len.is_some() {
            return;
        }
        self.len = Some(self.calc_len());
    }

    /// Calculates the length as integer are stored with variable length this operation is 'slow' and O(n) over the backends data!
    /// Call `init_len()` to precompute and temprorarily store the length in the current val.
    pub fn calc_len(&self) -> usize {
        let mut slice = self.backend.content_data();
        let mut len = 0;

        while !slice.is_empty() {
            let varint_width = varint_simd::decode_len::<T>(&slice).expect("unexpected");
            slice = &slice[varint_width..];
            len += 1;
        }

        len
    }
}

impl<B, T> CompressedNumberSequence<B, T>
where
    B: GrowableBackend,
    T: VarIntTarget,
{
    /// Pushes the given number into the number sequence, automatically compressing it.
    pub fn push(&mut self, number: T) -> Result<()> {
        let (datavec, len) = varint_simd::encode(number);
        let data = &datavec[..len as usize];
        if self.backend.free() < data.len() + 1 {
            self.backend.grow(data.len() + 1)?;
        }
        self.backend.push(data).unwrap();
        self.inc_len(1);
        Ok(())
    }
}

impl<B, T> Extend<T> for CompressedNumberSequence<B, T>
where
    B: GrowableBackend,
    T: VarIntTarget,
{
    #[inline]
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for i in iter {
            self.push(i).expect("Failed to push");
        }
    }
}

impl<B, T> Collection<T> for CompressedNumberSequence<B, T>
where
    B: Backend,
    T: VarIntTarget,
{
    type Iter<'a> = CompressedNumSeqIter<'a, T> where Self: 'a;
    type Iterator = OwnedCompressedNumSeqIterator<B, T>;

    #[inline]
    fn get(&self, index: usize) -> Result<T> {
        self.get(index).ok_or(Error::OutOfBounds)
    }

    #[inline]
    fn iter(&self) -> Self::Iter<'_> {
        self.iter()
    }

    #[inline]
    fn into_iter(self) -> Self::Iterator {
        OwnedCompressedNumSeqIterator::new(self)
    }
}

impl<B, T> GrowableCollection<T> for CompressedNumberSequence<B, T>
where
    B: GrowableBackend,
    T: VarIntTarget,
{
    #[inline]
    fn push(&mut self, item: T) -> Result<()> {
        self.push(item)
    }
}

impl<B, T> Creatable<B> for CompressedNumberSequence<B, T>
where
    B: GrowableBackend,
{
    #[inline]
    fn with_capacity(mut backend: B, capacity: usize) -> Result<Self> {
        backend.grow_to(capacity)?;
        Ok(Self {
            backend,
            p: PhantomData,
            len: Some(0),
        })
    }
}

impl<B, T> Initiable<B> for CompressedNumberSequence<B, T>
where
    B: Backend,
{
    #[inline]
    fn init(backend: B) -> Result<Self> {
        let len = match backend.len() {
            // if the backend is empty, we can assure that the number sequences length is 0.
            0 => Some(0),

            // if the backend has the length of 1, it clearly has at least one element but it can't hold two elements in
            // one byte so it must have one element.
            1 => Some(1),

            // For the other cases we can't tell a length so it needs to be computed. We want to keep the initialization
            // as low overhead as possible so we leave lengh calculation open to when its needed.
            _ => None,
        };

        Ok(Self {
            backend,
            p: PhantomData,
            len,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::backend::memory::test::make_mem_backend;
    use crate::components::indexed_file::IndexedFile;
    use crate::components::multi_file::MultiFile;
    use crate::traits::creatable::MemCreatable;
    use crate::traits::mtype::MType;

    #[test]
    fn create() {
        let mut cns: CompressedNumberSequence<_, u32> =
            CompressedNumberSequence::create_mem_with_capacity(10).unwrap();

        assert_eq!(cns.len(), 0);

        cns.push(19).unwrap();
        assert_eq!(cns.len(), 1);
        assert_eq!(cns.get(0), Some(19));

        cns.push(1263).unwrap();
        assert_eq!(cns.len(), 2);
        assert_eq!(cns.get(1), Some(1263));

        // We store less bytes than two u32 would need.
        assert!(cns.backend.len() < 2 * 4);
    }

    #[test]
    fn init() {
        let test_len = 1000usize;
        let mut backend = make_mem_backend(0);

        let mut cns: CompressedNumberSequence<_, u32> =
            CompressedNumberSequence::create(&mut backend).unwrap();
        cns.extend(0..test_len as u32);

        let cns: CompressedNumberSequence<_, u32> =
            CompressedNumberSequence::init(&mut backend).unwrap();
        for i in 0..test_len {
            assert_eq!(cns.get(i), Some(i as u32));
        }
        assert_eq!(cns.get(test_len), None);
        assert_eq!(cns.len(), test_len);
    }

    #[test]
    fn grow() {
        let mut list = MultiFile::create_mem_with_capacity(0).unwrap();
        list.insert_new_backend::<IndexedFile<_>>().unwrap();

        let mut ifile: IndexedFile<_> = list.get_backend_mut(0).unwrap();
        ifile.insert(&[]).unwrap();

        let mut entry = ifile.get_backend_mut(0).unwrap();

        let enc = varint_simd::encode(10u32);
        let end = entry.len();
        entry.grow(enc.1 as usize).unwrap();
        let new_data = &enc.0[..enc.1 as usize];
        entry.replace_same_len(end, new_data).unwrap();
        println!("{:?}", entry.raw_data());

        let cns: CompressedNumberSequence<_, u32> =
            CompressedNumberSequence::init(ifile.entry(0).unwrap()).unwrap();
        println!("{:?}", cns.backend.raw_data());
        let g = cns.get(0).unwrap();
        assert_eq!(g, 10);
    }
}

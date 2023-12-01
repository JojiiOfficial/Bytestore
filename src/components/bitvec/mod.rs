mod iter;

use crate::backend::growable::GrowableBackend;
use crate::backend::Backend;
use crate::components::bitvec::iter::BitVecIter;
use crate::traits::creatable::Creatable;
use crate::traits::initiable::Initiable;
use crate::{Error, Result};
use std::collections::Bound;
use std::ops::{Range, RangeBounds};

/// The first index holding bit data.
const FIRST_INDEX: usize = 8;

pub struct BitVec<B> {
    backend: B,
    len: usize,
}

impl<B> BitVec<B> {
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the byte index in the backend and the bit's index in the given byte.
    fn byte_index(&self, index: usize) -> Option<(usize, usize)> {
        self.oob_check(index).ok()?;
        Some(self.byte_index_unchecked(index))
    }

    /// Returns the byte index in the backend and the bit's index in the given byte.
    #[inline]
    fn byte_index_unchecked(&self, index: usize) -> (usize, usize) {
        let byte_index = (index / 8) + FIRST_INDEX;
        let bit_index = index % 8;
        (byte_index, bit_index)
    }

    #[inline]
    #[allow(dead_code)]
    fn bit_index(&self, index: usize) -> u8 {
        (index % 8) as u8
    }

    /// Checks for the bitvecs length bounds.
    #[inline]
    fn oob_check(&self, index: usize) -> Result<()> {
        if index >= self.len() {
            return Err(Error::OutOfBounds);
        }
        Ok(())
    }

    /// Returns the amount of bits stored in the last index or 0 if the map is empty.
    #[inline]
    fn last_index_occupied(&self) -> usize {
        if self.len == 0 {
            return 0;
        }
        let ocp = self.len % 8;
        if ocp == 0 {
            return 8;
        }
        ocp
    }

    /// Returns the amount of free bit slots in the last byte.
    #[inline]
    fn last_free(&self) -> usize {
        if self.len == 0 {
            return 0;
        }
        8 - self.last_index_occupied()
    }

    fn bit_range_to_bytes<R: RangeBounds<usize>>(
        &self,
        range: &R,
    ) -> Option<Range<(usize, usize)>> {
        let start = match range.start_bound() {
            Bound::Included(i) => self.byte_index(*i),
            Bound::Excluded(i) => self.byte_index(*i + 1),
            Bound::Unbounded => Some((0, 0)),
        }?;

        let end = match range.end_bound() {
            Bound::Included(i) => self.byte_index(*i),
            Bound::Excluded(i) => self.byte_index(i.checked_sub(1)?),
            Bound::Unbounded => Some(self.byte_index_unchecked(self.len.checked_sub(1)?)),
        }?;
        Some(start..end)
    }

    #[inline]
    fn bool_to_byte(b: bool) -> u8 {
        if b {
            u8::MAX
        } else {
            0
        }
    }
}

impl<B> BitVec<B>
where
    B: Backend,
{
    /// Returns the amount of bits that can be inserted in total into the BitVec without growing in size.
    #[inline]
    pub fn capacity(&self) -> usize {
        (self.backend.capacity() - FIRST_INDEX) * 8
    }

    /// Returns the amount of bits that can be inserted until the bitvec has to grow in size.
    #[inline]
    pub fn free(&self) -> usize {
        self.capacity() - self.len
    }

    /// Gets the value of the bool at `index`
    #[inline]
    pub fn get(&self, index: usize) -> Option<bool> {
        self.oob_check(index).ok()?;
        let (byte, mask) = self.get_unchecked(index);
        Some((byte & mask) == mask)
    }

    /// Returns the given byte containing `index` and its mask for a given index.
    pub fn get_unchecked(&self, index: usize) -> (u8, u8) {
        let (byte_idx, bit_index) = self.byte_index_unchecked(index);
        let byte = self.backend.get_single_unchecked(byte_idx);
        let mask = 1u8 << bit_index;
        (byte, mask)
    }

    /// Sets the value at `index` to `val`.
    pub fn set(&mut self, index: usize, val: bool) -> Result<()> {
        self.oob_check(index)?;
        let (byte_idx, bit_index) = self.byte_index_unchecked(index);
        let byte = self.backend.get_single_unchecked(byte_idx);
        let mask = 1u8 << bit_index;
        let old_val = (byte & mask) == mask;
        if old_val == val {
            return Ok(());
        }

        let new_byte = byte ^ mask;
        self.set_byte_unchecked(byte_idx, new_byte);
        Ok(())
    }

    /// Sets all bits in the given range to `val`. If the range exceeds the amount of items in the BitVec,
    /// an error gets returned.
    pub fn set_range(&mut self, range: impl RangeBounds<usize>, val: bool) -> Result<()> {
        let val = Self::bool_to_byte(val);
        let range = self.bit_range_to_bytes(&range).ok_or(Error::OutOfBounds)?;

        // Set first byte manually as it can start within that byte.
        let (sbyte_idx, sbit_idx) = range.start;
        let byte = self.get_byte_unchecked(sbyte_idx);
        let mask = u8::MAX << sbit_idx;
        let new_byte = (byte & !mask) | mask & val;
        self.set_byte_unchecked(sbyte_idx, new_byte);

        // Memset all parts until the last
        let (ebyte_idx, ebit_idx) = range.end;
        if ebyte_idx > sbyte_idx + 1 {
            let bytes_to_set = (ebyte_idx - sbyte_idx) - 1;
            let start = sbyte_idx + 1;
            let end = start + bytes_to_set;
            self.backend.fill(start..end, val)?;
        }

        // Manually adjust last byte
        if ebit_idx > 0 {
            let byte = self.get_byte_unchecked(ebyte_idx);
            let mask = u8::MAX >> (7 - ebit_idx);
            let new_byte = (byte & !mask) | mask & val;
            self.set_byte_unchecked(ebyte_idx, new_byte);
        }

        Ok(())
    }

    /// Returns the bits as `bool` in a newly allocated Vec.
    pub fn to_vec(&self) -> Vec<bool> {
        let mut out = Vec::with_capacity(self.len());

        for i in 0..self.len() {
            let (byte, mask) = self.get_unchecked(i);
            out.push((byte & mask) == mask);
        }

        out
    }

    /// Resets all bytes to false.
    #[inline]
    pub fn zero_all(&mut self) -> Result<()> {
        self.set_all(false)
    }

    /// Set all bits to `val` efficiently.
    pub fn set_all(&mut self, val: bool) -> Result<()> {
        if self.is_empty() {
            return Ok(());
        }
        let val = Self::bool_to_byte(val);
        self.backend.fill(FIRST_INDEX..self.backend.len(), val)?;
        Ok(())
    }

    /// Returns an iterator over all items in the `BitVec`.
    #[inline]
    pub fn iter(&self) -> BitVecIter<B> {
        BitVecIter::new(self)
    }

    #[inline]
    pub fn flush(&mut self) -> Result<()> {
        self.backend.flush()
    }

    /// Sets a byte at a given index to `val`.
    #[inline]
    fn set_byte_unchecked(&mut self, byte_index: usize, val: u8) {
        let index = self.backend.first_index() + byte_index;
        self.backend.data_mut()[index] = val;
    }

    #[inline]
    fn get_byte_unchecked(&self, index: usize) -> u8 {
        self.backend.get(index, 1).expect("Internal bug")[0]
    }
}

impl<B> BitVec<B>
where
    B: GrowableBackend,
{
    /// Pushes a new bit into the BitVec.
    pub fn push(&mut self, val: bool) -> Result<()> {
        self.alloc_n(1)?;
        let pos = self.len;
        self.len += 1;
        self.set(pos, val)
    }

    /// Add n more bits if the BitVec can't hold them. Does nothing if there is still enough space for `n` more bits.
    pub fn push_n(&mut self, n: usize, val: bool) -> Result<()> {
        let start_index = self.len();
        self.alloc_n(n)?;
        self.len += n;
        self.set_range(start_index.., val).unwrap();
        Ok(())
    }

    /// Increases the allocation so n more bits can be stored. Does nothing if there is still enough space for `n` more bits.
    /// Doesn't add zero elements.
    pub fn alloc_n(&mut self, n: usize) -> Result<bool> {
        let last_free = self.last_free();
        if last_free >= n {
            return Ok(false);
        }

        // All n that doesn't fit into the last byte
        let n = n - last_free;

        // Bytes need in total to store those n more elements
        let bytes_needed = n.div_ceil(8);

        // Amount of free items
        let be_free = self.backend.free();

        // Grow backend if free space doesn't fit
        if be_free < bytes_needed {
            let grow_len = bytes_needed - be_free;
            // Grow by factor 2 to reduce reallocations.
            let len = grow_len.max((self.backend.capacity() - 8) * 2);

            self.backend.grow(len)?;
        }

        self.backend.set_len(self.backend.len() + bytes_needed)?;

        Ok(true)
    }
}

impl<B> Extend<bool> for BitVec<B>
where
    B: GrowableBackend,
{
    fn extend<T: IntoIterator<Item = bool>>(&mut self, iter: T) {
        let iter = iter.into_iter();
        let (size, _) = iter.size_hint();
        if size > 0 {
            self.alloc_n(size).expect("Failed to allocate");
        }

        for i in iter {
            self.push(i).expect("Failed to insert");
        }
    }
}

impl<B> Extend<u8> for BitVec<B>
where
    B: GrowableBackend,
{
    #[inline]
    fn extend<T: IntoIterator<Item = u8>>(&mut self, iter: T) {
        self.extend(iter.into_iter().map(|i: u8| i == 1))
    }
}

impl<B> Creatable<B> for BitVec<B>
where
    B: GrowableBackend,
{
    fn with_capacity(mut backend: B, capacity: usize) -> Result<Self> {
        let real_cap = capacity.div_ceil(capacity.max(1));
        backend.grow_to(real_cap + 8)?;
        backend.push(&0usize.to_le_bytes())?;
        Ok(Self { backend, len: 0 })
    }
}

impl<B> Initiable<B> for BitVec<B>
where
    B: Backend,
{
    fn init(backend: B) -> Result<Self> {
        let blen = backend.get(0, 8).map_err(|_| Error::Initialization)?;
        let len = usize::from_le_bytes(blen.try_into().unwrap());
        Ok(Self { backend, len })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::backend::memory::test::make_mem_backend;
    use crate::traits::creatable::MemCreatable;
    use std::time::Instant;

    #[test]
    fn test_all() {
        let mut mem_backend = make_mem_backend(0);
        test_basic(&mut mem_backend);
        test_set_range(&mut mem_backend);
    }

    fn test_basic<B: GrowableBackend>(backend: &mut B) {
        backend.clear();
        let mut bvec = BitVec::create(backend).unwrap();
        assert_eq!(bvec.capacity(), 0);
        assert_eq!(bvec.free(), 0);

        bvec.push_n(2, false).unwrap();
        assert_eq!(bvec.capacity(), 8); // Can fit 8 bits now
        assert_eq!(bvec.get(0), Some(false));
        assert_eq!(bvec.get(1), Some(false));
        assert_eq!(bvec.get(2), None);
        assert_eq!(bvec.to_vec(), vec![false, false]);
        assert_eq!(bvec.to_vec(), bvec.iter().collect::<Vec<_>>());

        bvec.set(1, true).unwrap();
        assert_eq!(bvec.get(1), Some(true));
        bvec.set(0, true).unwrap();
        assert_eq!(bvec.get(0), Some(true));
        assert_eq!(bvec.to_vec(), vec![true, true]);
        assert_eq!(bvec.to_vec(), bvec.iter().collect::<Vec<_>>());

        assert!(bvec.set(2, true).is_err());

        bvec.push_n(1, true).unwrap();
        bvec.set(2, true).unwrap();
        assert_eq!(bvec.get(2), Some(true));
        assert_eq!(bvec.to_vec(), vec![true, true, true]);
        assert_eq!(bvec.to_vec(), bvec.iter().collect::<Vec<_>>());

        bvec.push(true).unwrap();

        assert_eq!(bvec.to_vec(), vec![true, true, true, true]);
        assert_eq!(bvec.to_vec(), bvec.iter().collect::<Vec<_>>());
    }

    #[test]
    fn test_bug() {
        let mut bv = BitVec::create_mem_with_capacity(1000).unwrap();
        bv.extend((0..10_000).map(|i| i % 43 == 0));
        bv.set(2361, true).unwrap();
    }

    fn test_set_range<B: GrowableBackend>(backend: &mut B) {
        backend.clear();
        let mut bvec = BitVec::create(backend).unwrap();
        let start = Instant::now();
        bvec.extend((0..1_000_000).map(|i| i % 3 == 0));
        println!("{} elements took: {:?}", bvec.len(), start.elapsed());

        for (pos, val) in (0..bvec.len()).enumerate() {
            assert_eq!(bvec.get(pos), Some(val % 3 == 0));
        }
        assert_eq!(bvec.to_vec(), bvec.iter().collect::<Vec<_>>());

        bvec.set_range(10..bvec.len() - 5, true).unwrap();
        for (pos, val) in (0..bvec.len()).enumerate() {
            if pos < 10 {
                assert_eq!(bvec.get(pos), Some(val % 3 == 0));
            } else if pos < bvec.len() - 5 {
                assert_eq!(bvec.get(pos), Some(true));
            } else {
                assert_eq!(bvec.get(pos), Some(val % 3 == 0));
            }
        }
        assert_eq!(bvec.to_vec(), bvec.iter().collect::<Vec<_>>());

        bvec.zero_all().unwrap();
        for (pos, _) in (0..bvec.len()).enumerate() {
            assert_eq!(bvec.get(pos), Some(false));
        }
        assert_eq!(bvec.to_vec(), bvec.iter().collect::<Vec<_>>());

        bvec.set_all(true).unwrap();
        for (pos, _) in (0..bvec.len()).enumerate() {
            assert_eq!(bvec.get(pos), Some(true));
        }
        assert_eq!(bvec.to_vec(), bvec.iter().collect::<Vec<_>>());
    }
}

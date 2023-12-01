use crate::deser::{deserialize_impl, serialize_impl};
use crate::error::Error;
use crate::utils::ranges_overlap;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::ops::Range;

pub mod base;
pub mod full;
pub mod growable;
pub mod memory;
pub mod mmap_mut;
pub mod mmap;

pub trait Backend {
    /// Should return the whole data in bytes.
    fn data(&self) -> &[u8];
    /// Should return the whole data in bytes mutable.
    fn data_mut(&mut self) -> &mut [u8];

    /// Should return the first writable index (eg without prepending headers that may be contained in `data()`.
    fn first_index(&self) -> usize;

    /// Returns the amount of bytes that have been written (after `first_index()`)
    fn len(&self) -> usize;
    /// Sets the amonut of bytes written after `first_index()`
    fn set_len(&mut self, len: usize) -> Result<(), Error>;

    #[inline]
    fn inc_len(&mut self, amount: usize) -> Result<(), Error> {
        let len = self.len();
        self.set_len(len + amount)
    }

    /// Returns the last index that still holds set data.
    #[inline]
    fn last_index(&self) -> usize {
        self.first_index() + self.len()
    }

    /// Performs a bounds check for the data only.
    #[inline]
    fn check_len_oob(&self, index: usize) -> Result<(), Error> {
        if index > self.last_index() {
            return Err(Error::OutOfBounds);
        }

        Ok(())
    }

    /// Performs a bounds check for the whole file (including the empty part at the end)
    fn check_capacity_oob(&self, index: usize) -> Result<(), Error> {
        if index > self.end_index() + 1 {
            return Err(Error::OutOfBounds);
        }
        Ok(())
    }

    /// Returns the data of the backends content which is within between first_index() and len()
    #[inline]
    fn content_data(&self) -> &[u8] {
        let first = self.first_index();
        let end = self.last_index();
        &self.data()[first..end]
    }

    /// Returns the data of the backends content which is within between first_index() and len()
    #[inline]
    fn content_data_mut(&mut self) -> &mut [u8] {
        let first = self.first_index();
        let end = self.last_index();
        &mut self.data_mut()[first..end]
    }

    /// Pushes raw data
    fn push(&mut self, data: &[u8]) -> Result<usize, Error> {
        let pos = self.len();
        if data.is_empty() {
            return Ok(pos);
        }

        let next_slice = self.next_free_slice(data.len())?;
        next_slice.copy_from_slice(data);
        self.set_len(self.len() + data.len())?;
        Ok(pos)
    }

    /// Pushes raw data
    fn push_fill(&mut self, data: u8, len: usize) -> Result<usize, Error> {
        let pos = self.len();
        if len == 0 {
            return Ok(pos);
        }

        let next_slice = self.next_free_slice(len)?;
        // next_slice.copy_from_slice(data);
        next_slice.fill(data);
        self.set_len(self.len() + len)?;
        Ok(pos)
    }

    /// Pushes a typed value
    fn push_t<T: Serialize>(&mut self, data: &T) -> Result<(usize, usize), Error> {
        let data = serialize_impl(data)?;
        let idx = self.push(&data)?;
        Ok((idx, data.len()))
    }

    /// Gets the data at index..index+len.
    fn get(&self, index: usize, len: usize) -> Result<&[u8], Error> {
        let start = self.get_index(index);
        let end = start + len;
        self.check_len_oob(end)?;
        Ok(&self.data()[start..end])
    }

    /// Gets a single byte at the given index.
    #[inline]
    fn get_single(&self, index: usize) -> Result<u8, Error> {
        let start = self.get_index(index);
        self.check_len_oob(index)?;
        Ok(self.data()[start])
    }

    /// Gets a single byte at the given index.
    #[inline]
    fn get_single_unchecked(&self, index: usize) -> u8 {
        let start = self.get_index(index);
        self.data()[start]
    }

    /// Gets the data at index..index+len.
    #[inline]
    fn get_unchecked(&self, index: usize, len: usize) -> &[u8] {
        let start = self.get_index(index);
        let end = start + len;
        &self.data()[start..end]
    }

    /// Gets the data at index..index+len mutable.
    fn get_mut(&mut self, index: usize, len: usize) -> Result<&mut [u8], Error> {
        let start = self.get_index(index);
        let end = start + len;
        self.check_len_oob(end)?;
        Ok(&mut self.data_mut()[start..end])
    }

    /// Gets typed at a given position
    #[inline]
    fn get_t<T: DeserializeOwned>(&self, index: usize, len: usize) -> Result<T, Error> {
        deserialize_impl(self.get(index, len)?)
    }

    /*
    /// Swaps the data given by their ranges.
    fn swap(&mut self, a: Range<usize>, b: Range<usize>) -> Result<(), Error> {
        if a.is_empty() || b.is_empty() {
            return Ok(());
        }

        let a_end_idx = self.get_index(a.end);
        let b_end_idx = self.get_index(b.end);
        self.check_len_oob(a_end_idx)?;
        self.check_len_oob(b_end_idx)?;

        if a.len() == b.len() {
            return self.swap_same_len(a.start, b.start, a.len());
        }

        // Ranges are overlapping
        if ranges_overlap(&a, &b) {
            return Err(Error::OutOfBounds);
        }

        let (smaller, bigger) = if a.len() > b.len() { (b, a) } else { (a, b) };
        self.swap_same_len(smaller.start, bigger.start, smaller.len())?;
        let copied = smaller.len();
        // let rest = bigger.len() - copied;

        Ok(())
    }
     */

    /// Swaps the range `a` with `b`. Returns an error if the ranges are empty, overlap or differ in Size.
    fn swap_same_len(&mut self, a: usize, b: usize, len: usize) -> Result<(), Error> {
        if ranges_overlap(&(a..a + len), &(b..b + len)) || len == 0 {
            return Err(Error::OutOfBounds);
        }

        let a = self.get_index(a);
        let b = self.get_index(b);

        let (a, b) = if a > b { (b, a) } else { (a, b) };

        let (first, second) = self.data_mut().split_at_mut(b);
        first[a..a + len].swap_with_slice(&mut second[..len]);
        Ok(())
    }

    /// Replaces the data at index..index+len with `data`. If `len == 0 ` `data` gets inserted at `index`
    fn replace_fill(
        &mut self,
        index: usize,
        len: usize,
        data: u8,
        fill_len: usize,
    ) -> Result<usize, Error> {
        if len == fill_len {
            return self.replace_same_len_fill(index, data, fill_len);
        }

        let start_index = self.get_index(index);

        self.check_len_oob(start_index)?;

        let move_start_index = start_index + len;

        // no need to move any data if it exceeds the range of actual data
        if move_start_index < self.end_index() {
            let move_end = self.last_index();
            let new_start_index = start_index + fill_len;

            self.data_mut()
                .copy_within(move_start_index..move_end, new_start_index);
        }

        if fill_len > 0 {
            let end_index = start_index + fill_len;
            self.check_capacity_oob(end_index)?;

            // self.data_mut()[start_index..end_index].copy_from_slice(data);
            self.data_mut()[start_index..end_index].fill(data);
        }

        let len = len.min(self.len());
        let diff = len.abs_diff(fill_len);
        if len > fill_len {
            self.set_len(self.len() - diff)?;
        } else {
            self.set_len(self.len() + diff)?;
        }

        Ok(diff)
    }

    /// Replaces the data at index..index+len with `data`. If `len == 0 ` `data` gets inserted at `index`
    fn replace(&mut self, index: usize, len: usize, data: &[u8]) -> Result<usize, Error> {
        if len == data.len() {
            return self.replace_same_len(index, data);
        }

        let start_index = self.get_index(index);

        self.check_len_oob(start_index)?;

        let move_start_index = start_index + len;

        // no need to move any data if it exceeds the range of actual data
        if move_start_index < self.end_index() {
            let move_end = self.last_index();
            let new_start_index = start_index + data.len();

            self.data_mut()
                .copy_within(move_start_index..move_end, new_start_index);
        }

        if !data.is_empty() {
            let end_index = start_index + data.len();
            self.check_capacity_oob(end_index)?;
            self.data_mut()[start_index..end_index].copy_from_slice(data);
        }

        let len = len.min(self.len());
        let diff = len.abs_diff(data.len());
        if len > data.len() {
            self.set_len(self.len() - diff)?;
        } else {
            self.set_len(self.len() + diff)?;
        }

        Ok(diff)
    }


    fn replace_t<T: Serialize>(
        &mut self,
        index: usize,
        len: usize,
        data: &T,
    ) -> Result<usize, Error> {
        let data = serialize_impl(data)?;
        self.replace(index, len, &data)?;
        Ok(data.len())
    }

    /// Replaces the item at `index` with the serialized data `data` and returns the amount of writes replaced.
    fn replace_same_len_t<T: Serialize>(&mut self, index: usize, data: &T) -> Result<usize, Error> {
        let data = serialize_impl(data)?;
        self.replace_same_len(index, &data)?;
        Ok(data.len())
    }

    /// Removes the data at the given index until `len`
    #[inline]
    fn remove(&mut self, index: usize, len: usize) -> Result<(), Error> {
        self.replace(index, len, &[])?;
        Ok(())
    }

    /// Returns the amount of bytes that can be added (without need of growing the data)
    #[inline]
    fn capacity(&self) -> usize {
        self.data().len() - self.first_index()
    }

    /// Returns the last index at the end of the slice that can still be written
    #[inline]
    fn end_index(&self) -> usize {
        self.data().len().saturating_sub(1)
    }

    /// Gets the internal position
    #[inline]
    fn get_index(&self, index: usize) -> usize {
        self.first_index() + index
    }

    /// Replace index..index+data.len() bytes with `data` `fill_len` times
    fn replace_same_len_fill(
        &mut self,
        index: usize,
        data: u8,
        fill_len: usize,
    ) -> Result<usize, Error> {
        let start = self.get_index(index);
        let end = start + fill_len;
        self.check_capacity_oob(end)?;
        self.data_mut()[start..end].fill(data);

        if end > self.last_index() {
            let diff = end - self.last_index();
            self.set_len(self.len() + diff).unwrap();
            return Ok(diff);
        }

        Ok(0)
    }

    /// Replace index..index+data.len() bytes with `data`
    fn replace_same_len(&mut self, index: usize, data: &[u8]) -> Result<usize, Error> {
        let start = self.get_index(index);
        self.replace_same_len_direct(start, data)
    }

    /// Replace index..index+data.len() bytes with `data`
    fn replace_same_len_direct(&mut self, index: usize, data: &[u8]) -> Result<usize, Error> {
        let end = index + data.len();
        self.check_capacity_oob(end)?;
        self.data_mut()[index..end].copy_from_slice(data);

        if end > self.last_index() {
            let diff = end - self.last_index();
            self.set_len(self.len() + diff).unwrap();
            return Ok(diff);
        }

        Ok(0)
    }

    /// Returns the next free subslice of length `len`.
    fn next_free_slice(&mut self, len: usize) -> Result<&mut [u8], Error> {
        let start = self.get_index(self.len());
        let end = start + len;
        self.check_capacity_oob(end)?;
        Ok(&mut self.data_mut()[start..end])
    }

    /// Fills the given range with `val`. This range has to be within the length bounds, so only valid data can be filled.
    fn fill(&mut self, range: Range<usize>, val: u8) -> Result<(), Error> {
        if range.is_empty() {
            return Ok(());
        }

        let start = self.get_index(range.start);
        let end = self.get_index(range.end);
        self.check_len_oob(end)?;
        self.data_mut()[start..end].fill(val);
        Ok(())
    }

    fn clear(&mut self) {
        // set_len only throws an error if new len is bigger than capacity but 0 can't be bigger
        // than capacity.
        self.set_len(0).unwrap()
    }

    /// Returns `true` if the Backend doesn't hold any data.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns `true` if the backend is full and need to be resized.
    #[inline]
    fn is_full(&self) -> bool {
        self.len() == self.capacity()
    }

    /// Returns the amount of bytes that are still free.
    #[inline]
    fn free(&self) -> usize {
        self.capacity() - self.len()
    }

    /// Writes written data to the underlying medium.
    #[inline]
    fn flush(&mut self) -> Result<(), Error> {
        self.flush_range_impl(0, self.data().len())
    }

    /// Flushes a given range. Don't overwrite this method, implement `flush_range_impl`!
    #[inline]
    fn flush_range(&mut self, start: usize, len: usize) -> Result<(), Error> {
        if start + len > self.data().len() {
            return Err(Error::OutOfBounds);
        }
        self.flush_range_impl(start, len)
    }

    /// Flushes a given range. Implement this for backends. No bounds checking needed.
    #[inline]
    fn flush_range_impl(&mut self, _start: usize, _len: usize) -> Result<(), Error> {
        Ok(())
    }

    /// Moves all bytes in a given range to a new index. This works on the raw data indices, without taking first_index()
    /// into account!
    #[inline]
    fn move_range_to(&mut self, index: usize, len: usize, new_index: usize) -> Result<(), Error> {
        self.check_len_oob(index + len)?;
        self.check_len_oob(new_index + len)?;
        let move_end = index + len;
        self.data_mut()
            .copy_within(index..move_end, new_index);
        Ok(())
    }
}

impl<B> Backend for &mut B
    where
        B: Backend,
{
    #[inline]
    fn data(&self) -> &[u8] {
        (**self).data()
    }

    #[inline]
    fn data_mut(&mut self) -> &mut [u8] {
        (*self).data_mut()
    }

    #[inline]
    fn first_index(&self) -> usize {
        (**self).first_index()
    }

    #[inline]
    fn len(&self) -> usize {
        (**self).len()
    }

    #[inline]
    fn set_len(&mut self, len: usize) -> Result<(), Error> {
        (*self).set_len(len)
    }
}

impl<'a> Backend for &'a mut [u8] {
    #[inline]
    fn data(&self) -> &[u8] {
        &self
    }

    #[inline]
    fn data_mut(&mut self) -> &mut [u8] {
        self
    }

    #[inline]
    fn first_index(&self) -> usize {
        0
    }

    #[inline]
    fn len(&self) -> usize {
        <[u8]>::len(self)
    }

    #[inline]
    fn set_len(&mut self, _len: usize) -> Result<(), Error> {
        Ok(())
    }
}

impl<'a> Backend for &'a [u8] {
    #[inline]
    fn data(&self) -> &[u8] {
        &self
    }

    fn data_mut(&mut self) -> &mut [u8] {
        panic!("&[u8] backend is not mutable")
    }

    #[inline]
    fn first_index(&self) -> usize {
        0
    }

    #[inline]
    fn len(&self) -> usize {
        <[u8]>::len(self)
    }

    #[inline]
    fn set_len(&mut self, _len: usize) -> Result<(), Error> {
        Ok(())
    }
}

#[cfg(test)]
pub mod test {
    use crate::backend::base::BaseBackend;
    use crate::backend::Backend;
    use crate::error::Error;
    use std::ops::DerefMut;

    pub fn be_push<S: DerefMut<Target=[u8]>>(backend: &mut BaseBackend<S>)
        where
            BaseBackend<S>: Backend,
    {
        backend.clear();

        let data: &[u8] = &[1, 2, 3];
        backend.push(data).unwrap();
        assert_eq!(backend.get(0, 3), Ok(data));
        assert_eq!(backend.len(), data.len());

        let new_data: &[u8] = &[9, 1, 5, 6, 12, 61, 156];
        backend.push(new_data).unwrap();
        assert_eq!(backend.get(3, new_data.len()), Ok(new_data));
        assert_eq!(backend.len(), data.len() + new_data.len());
    }

    pub fn be_clear<B: Backend>(backend: &mut B) {
        assert!(backend.is_empty());
        let data: &[u8] = &[10, 10, 10];
        backend.push(data).unwrap();
        assert_eq!(backend.len(), 3);
        assert!(!backend.is_empty());
        assert_eq!(backend.get(0, 3), Ok(data));
        backend.clear();
        assert_eq!(backend.len(), 0);
        assert!(backend.is_empty());
        assert_eq!(backend.get(0, 3), Err(Error::OutOfBounds));
    }

    pub fn be_replace<S: DerefMut<Target=[u8]>>(backend: &mut BaseBackend<S>)
        where
            BaseBackend<S>: Backend,
    {
        backend.clear();

        let init_data = &[1, 2, 4, 0];
        backend.push(init_data).unwrap();

        backend.replace(1, 1, &[9, 9]).unwrap();
        assert_eq!(backend.get(0, 5), Ok(&[1, 9, 9, 4, 0][..]));
        assert_eq!(backend.len(), 5);

        backend.replace(0, 0, &[6]).unwrap();
        assert_eq!(backend.get(0, 6), Ok(&[6, 1, 9, 9, 4, 0][..]));
        assert_eq!(backend.len(), 6);

        backend.replace(0, 6, &[9, 9, 9, 9, 9, 9]).unwrap();
        assert_eq!(backend.get(0, 6), Ok(&[9, 9, 9, 9, 9, 9][..]));
        assert_eq!(backend.len(), 6);

        let mut sub = backend.sub_backend_mut(10).unwrap();
        assert_eq!(sub.capacity(), 10);
        assert_eq!(sub.data().len(), 18);
        assert_eq!(sub.len(), 6);

        let zeroes = &[0; 10][..];
        sub.replace(0, 10, zeroes).unwrap();
        assert_eq!(sub.len(), 10);
        assert_eq!(sub.capacity(), 10);
        assert_eq!(sub.get(0, 10), Ok(zeroes));

        // Replace last index
        sub.replace(9, 1, &[9]).unwrap();
        assert_eq!(sub.get(9, 1), Ok(&[9][..]));
        assert_eq!(sub.capacity(), 10);

        let oobreplace = sub.replace(10, 1, &[4]);
        assert_eq!(oobreplace, Err(Error::OutOfBounds));

        // Shrink resize
        sub.replace(0, 10, &[]).unwrap();
        assert_eq!(sub.len(), 0);
        assert_eq!(sub.capacity(), 10);

        // Extend again
        sub.replace(0, 10, &[1, 2, 3]).unwrap();
        assert_eq!(sub.len(), 3);
        assert_eq!(sub.get(0, 3), Ok(&[1, 2, 3][..]));
        assert_eq!(sub.get(3, 1), Err(Error::OutOfBounds));

        // [1,2,3,0,0,0,0,0,0,0]
        sub.replace(2, 0, &[9, 9, 9]).unwrap();
        assert_eq!(sub.get(0, 6), Ok(&[1, 2, 9, 9, 9, 3][..]));
    }

    pub fn be_remove<B: Backend>(backend: &mut B) {
        backend.clear();

        backend.push(&[1, 2, 3]).unwrap();

        assert_eq!(backend.get(0, 3), Ok(&[1, 2, 3][..]));
        assert_eq!(backend.len(), 3);

        backend.remove(1, 2).unwrap();
        assert_eq!(backend.get(0, 1), Ok(&[1][..]));
        assert_eq!(backend.len(), 1);
    }

    pub fn be_fill<B: Backend>(backend: &mut B) {
        backend.clear();
        let len = backend.capacity();
        let mut data = vec![];
        for i in 0..len {
            let item = (i % 255) as u8;
            data.push(vec![item]);
            backend.push(&[item]).unwrap();
        }
        assert!(backend.is_full());
        assert_eq!(backend.free(), 0);
        assert_eq!(backend.len(), backend.capacity());
    }

    pub fn be_stresstest<B: Backend>(backend: &mut B) {
        backend.clear();

        let mut data = vec![];
        for i in 0..10_000 {
            let txt = format!("{i}_{i}-{i}").repeat(i % 10);
            data.push(txt.clone());
            backend.push(txt.as_bytes()).unwrap();
        }

        let mut pos = 0;
        for i in data.iter() {
            let r = backend.get(pos, i.as_bytes().len()).unwrap();
            assert_eq!(std::str::from_utf8(r).unwrap(), i);
            pos += r.len();
        }
        assert_eq!(backend.len(), pos);
    }
}

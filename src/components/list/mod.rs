// Still in todo
// mod cint;

pub mod iter;
mod presets;

pub use presets::*;
use std::collections::Bound;

use crate::backend::growable::GrowableBackend;
use crate::backend::memory::MemoryBackend;
use crate::backend::Backend;
use crate::components::list::iter::ListIter;
use crate::deser::{deserialize_impl, serialize_impl};
use crate::traits::creatable::{Creatable, MemCreatable};
use crate::traits::initiable::Initiable;
use crate::utils::smallest_two_power_for;
use crate::{Error, Result};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::marker::PhantomData;
use std::ops::RangeBounds;

/// A Vec<T> like type where each entry `T` de/serializes with the same amount of bytes `N` > 0.
/// This allows us to store the items storage efficiently, without the need of an index that holds
/// the byte ranges for each entry.
///
/// # Panics
/// This datastructure expects N > 0 so initializing `List<B, T, 0>` will cause a panic!
pub struct List<B, T, const N: usize> {
    backend: B,
    len: usize,
    _p1: PhantomData<T>,
}

impl<B, T, const N: usize> List<B, T, N> {
    /// Returns the amount of items in the List.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the List is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns an iterator iterating over all elements of the list.
    #[inline]
    pub fn iter(&self) -> ListIter<B, T, N> {
        ListIter::new(self)
    }

    /// Returns the index of `index` in the backend slice.
    #[inline]
    fn byte_index(index: usize) -> usize {
        index * N
    }

    /// Returns an error if `index` is not within the bounds of the items of the list.
    fn check_oob(&self, index: usize) -> Result<()> {
        if index >= self.len() {
            return Err(Error::OutOfBounds);
        }
        Ok(())
    }
}

impl<B, T, const N: usize> List<B, T, N>
where
    B: Backend,
{
    /// Gets an entry at `index` as bytes.
    #[inline]
    pub fn get_raw(&self, index: usize) -> Result<&[u8]> {
        let index = Self::byte_index(index);
        self.backend.get(index, N)
    }

    /// Gets an entry at `index` as mutable bytes.
    #[inline]
    pub fn get_raw_mut(&mut self, index: usize) -> Result<&mut [u8]> {
        let index = Self::byte_index(index);
        self.backend.get_mut(index, N)
    }

    /// Sets the value of an element at `pos` to `item`.
    pub fn set_raw(&mut self, index: usize, data: &[u8]) -> Result<()> {
        self.check_oob(index)?;
        if data.len() != N {
            return Err(Error::UnexpectedValue);
        }

        let index = Self::byte_index(index);
        self.backend.replace_same_len(index, data)?;
        Ok(())
    }

    /// Returns the amount of items that can be pushed into the list without regrowing.
    #[inline]
    pub fn free(&self) -> usize {
        self.backend.free() / N
    }

    /// Returns the total capacity of the List. This includes the amount of items that can be pushed
    /// and the amount of items that already have been pushed.
    pub fn capacity(&self) -> usize {
        self.backend.capacity() / N
    }

    /// Removes all items from the List, preserving the allocated space.
    pub fn clear(&mut self) {
        self.backend.clear();
        self.len = 0;
    }

    /// Removes an item, shifting all following items to the left.
    pub fn remove(&mut self, index: usize) -> Result<()> {
        self.check_oob(index)?;
        let byte_offset = Self::byte_index(index);
        self.backend.replace(byte_offset, N, &[])?;
        self.len -= 1;
        Ok(())
    }

    #[inline]
    pub fn flush(&mut self) -> Result<()> {
        self.backend.flush()
    }

    /// Sets all bytes that are filled with values to `val`.
    pub(crate) fn mem_set_r(&mut self, range: impl RangeBounds<usize>, val: u8) -> Result<()> {
        if self.is_empty() {
            return Ok(());
        }

        let start = match range.start_bound() {
            Bound::Included(i) => *i,
            Bound::Excluded(i) => *i + 1,
            Bound::Unbounded => 0,
        };

        let end_byte = Self::byte_index(self.len());
        let end = match range.end_bound() {
            Bound::Included(i) => *i,
            Bound::Excluded(i) => end_byte.saturating_sub(*i),
            Bound::Unbounded => end_byte,
        };

        let range = start..end;
        self.backend.fill(range, val)?;
        Ok(())
    }

    /// Sets all bytes that are filled with values to `val`.
    pub(crate) fn mem_set(&mut self, val: u8) -> Result<()> {
        if self.is_empty() {
            return Ok(());
        }

        self.mem_set_r(.., val)?;
        // let end = Self::byte_index(self.len());
        // self.backend.fill(0..end, val)?;
        Ok(())
    }

    pub(crate) fn set_len(&mut self, len: usize) -> Result<()> {
        let blen = Self::byte_index(len);
        if blen > self.backend.capacity() {
            return Err(Error::OutOfBounds);
        }

        self.backend.set_len(blen)?;
        self.len = len;

        Ok(())
    }
}

impl<B, T, const N: usize> List<B, T, N>
where
    B: Backend,
    T: DeserializeOwned,
{
    /// Gets an entry at `index`.
    #[inline]
    pub fn get(&self, index: usize) -> Result<T> {
        let data = self.get_raw(index)?;
        deserialize_impl(data)
    }

    /// Tries to remove the last element. If there was an element to remove `true` gets returned.
    pub fn pop(&mut self) -> Result<bool> {
        if self.is_empty() {
            return Ok(false);
        }
        self.len -= 1;
        self.backend.set_len(self.backend.len() - N)?;
        Ok(true)
    }
}

impl<B, T, const N: usize> List<B, T, N>
where
    T: Serialize,
    B: Backend,
{
    /// Sets the value of an element at `pos` to `item`.
    #[inline]
    pub fn set(&mut self, index: usize, item: &T) -> Result<()> {
        self.set_raw(index, &serialize_impl(item)?)
    }
}

impl<B, T, const N: usize> List<B, T, N>
where
    B: GrowableBackend,
    T: Serialize,
{
    /// Pushes a new item into list.
    #[inline]
    pub fn push(&mut self, item: &T) -> Result<()> {
        self.push_raw(&serialize_impl(item)?)
    }

    /// Inserts an item at the given `index` in the list.
    #[inline]
    pub fn insert(&mut self, index: usize, item: &T) -> Result<()> {
        self.insert_raw(index, &serialize_impl(item)?)
    }
}

impl<B, T, const N: usize> List<B, T, N>
where
    B: GrowableBackend,
{
    /// Pushes a new item into list in form of raw data.
    pub fn push_raw(&mut self, data: &[u8]) -> Result<()> {
        if data.len() != N {
            return Err(Error::UnexpectedValue);
        }

        // Grow when necessary.
        if self.free() == 0 {
            self.grow()?;
        }

        self.backend.push(data)?;
        self.len += 1;
        Ok(())
    }

    /// Inserts data at the given `index` in the list.
    pub fn insert_raw(&mut self, index: usize, data: &[u8]) -> Result<()> {
        if self.free() == 0 {
            self.grow_for(1)?;
        }

        if data.len() != N {
            return Err(Error::UnexpectedValue);
        }

        let byte_offset = Self::byte_index(index);
        self.backend.replace(byte_offset, 0, data)?;
        self.len += 1;
        Ok(())
    }
}

impl<B, T, const N: usize> List<B, T, N>
where
    B: GrowableBackend,
{
    /// Grows the lists backend for more items. This means growing to twice the size of the current capacity.
    #[inline]
    pub fn grow(&mut self) -> Result<()> {
        // Make sure to actually grow the list for at least one element as self.len() can be 0.
        self.grow_for(self.len().max(1))
    }

    /// Grows the list for additional `items` new entries.
    pub fn grow_for_exact(&mut self, items: usize) -> Result<()> {
        if items == 0 {
            return Ok(());
        }

        self.backend.grow_to(Self::byte_index(self.len() + items))?;
        Ok(())
    }

    /// Grows the list for at least additional `items` new entries.
    pub fn grow_for(&mut self, items: usize) -> Result<()> {
        if items == 0 {
            return Ok(());
        }

        // List needs to hold `items` additional items
        let need_hold = items + self.len();
        let pow = smallest_two_power_for(need_hold);
        self.backend.grow_to(Self::byte_index(2usize.pow(pow)))?;
        Ok(())
    }
}

impl<B, T, const N: usize> Creatable<B> for List<B, T, N>
where
    B: GrowableBackend,
{
    fn with_capacity(mut backend: B, capacity: usize) -> Result<Self> {
        assert!(N > 0);
        let bytes_needed = capacity * N;
        backend.grow_to(bytes_needed)?;
        Ok(Self {
            len: 0,
            backend,
            _p1: PhantomData,
        })
    }
}

impl<B, T, const N: usize> Initiable<B> for List<B, T, N>
where
    B: Backend,
{
    #[inline]
    fn init(backend: B) -> Result<Self> {
        assert!(N > 0);
        if backend.len() % N != 0 {
            return Err(Error::Initialization);
        }

        let len = backend.len() / N;
        Ok(Self {
            len,
            backend,
            _p1: PhantomData,
        })
    }
}

impl<B, T, const N: usize> Extend<T> for List<B, T, N>
where
    B: GrowableBackend,
    T: Serialize,
{
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        let iter = iter.into_iter();
        let (size, _) = iter.size_hint();

        if size > 0 && self.free() < size {
            self.grow_for(size).expect("Failed to grow");
        }

        for i in iter {
            self.push(&i).expect("Failed to insert");
        }
    }
}

impl<T, const N: usize> FromIterator<T> for List<MemoryBackend, T, N>
where
    T: Serialize,
{
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let mut list = List::create_mem_with_capacity(iter.size_hint().0 * 4)
            .expect("Failed to allocate list");
        list.extend(iter);
        list
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::backend::memory::test::make_mem_backend;
    use crate::backend::mmap_mut::test::make_mmap_backend;
    use crate::traits::creatable::MemCreatable;

    #[test]
    fn grow_few() {
        let mut list = ListU32::create_mem_with_capacity(0).unwrap();
        assert_eq!(list.capacity(), 0);
        list.push(&4).unwrap();
        assert_eq!(list.capacity(), 2);
        list.push(&4).unwrap();
        assert_eq!(list.capacity(), 2);
        list.push(&4).unwrap();
        assert_eq!(list.capacity(), 4);
        list.push(&4).unwrap();
        assert_eq!(list.capacity(), 4);
        list.push(&4).unwrap();
        assert_eq!(list.capacity(), 8);
    }

    #[test]
    fn test_all() {
        let mut backend = make_mem_backend(1);
        insert_get(&mut backend);
        grow(&mut backend);
        insert_at(&mut backend);
        mem_set(&mut backend);

        let mut backend = make_mmap_backend("list_test", 1);
        insert_get(&mut backend);
        grow(&mut backend);
        insert_at(&mut backend);
        mem_set(&mut backend);
    }

    fn insert_get<B: GrowableBackend>(backend: &mut B) {
        backend.clear();
        let mut list = ListU32::create(backend).unwrap();

        list.push(&1).unwrap();
        list.push(&9).unwrap();
        list.push(&16).unwrap();

        assert_eq!(list.len(), 3);
        assert_eq!(list.get(0), Ok(1));
        assert_eq!(list.get(1), Ok(9));
        assert_eq!(list.get(2), Ok(16));
        assert_eq!(list.get(3), Err(Error::OutOfBounds));

        list.set(0, &7).unwrap();
        assert_eq!(list.get(0), Ok(7));
        assert_eq!(list.get(1), Ok(9));

        assert_eq!(list.set(3, &10), Err(Error::OutOfBounds));
    }

    fn insert_at<B: GrowableBackend>(backend: &mut B) {
        backend.clear();
        let mut list = ListU32::create(backend).unwrap();
        list.push(&1).unwrap();
        list.push(&2).unwrap();
        list.push(&3).unwrap();
        list.push(&4).unwrap();
        assert_eq!(list.iter().collect::<Vec<_>>(), vec![1, 2, 3, 4]);

        list.insert(0, &0).unwrap();
        assert_eq!(list.iter().collect::<Vec<_>>(), vec![0, 1, 2, 3, 4]);

        list.insert(0, &0).unwrap();
        assert_eq!(list.iter().collect::<Vec<_>>(), vec![0, 0, 1, 2, 3, 4]);

        list.remove(0).unwrap();
        assert_eq!(list.iter().collect::<Vec<_>>(), vec![0, 1, 2, 3, 4]);

        list.insert(1, &10).unwrap();
        assert_eq!(list.iter().collect::<Vec<_>>(), vec![0, 10, 1, 2, 3, 4]);

        list.remove(1).unwrap();
        assert_eq!(list.iter().collect::<Vec<_>>(), vec![0, 1, 2, 3, 4]);

        list.insert(list.len(), &99).unwrap();
        assert_eq!(list.iter().collect::<Vec<_>>(), vec![0, 1, 2, 3, 4, 99]);

        for _ in 0..list.len() {
            list.remove(0).unwrap();
        }
        assert_eq!(list.len(), 0);
        assert_eq!(list.get(0), Err(Error::OutOfBounds));
        assert_eq!(list.iter().collect::<Vec<_>>(), vec![]);
        assert_eq!(list.iter().count(), 0);
        assert!(list.capacity() >= 4);
    }

    fn mem_set<B: GrowableBackend>(backend: &mut B) {
        backend.clear();
        let mut list = ListU32::create(backend).unwrap();
        list.push(&1).unwrap();
        list.push(&2).unwrap();
        list.push(&3).unwrap();
        list.push(&4).unwrap();

        let old_len = list.len();
        assert_eq!(list.iter().collect::<Vec<_>>(), vec![1, 2, 3, 4]);
        assert_eq!(old_len, 4);

        list.mem_set(0).unwrap();
        assert_eq!(list.iter().collect::<Vec<_>>(), vec![0, 0, 0, 0]);
        assert_eq!(old_len, 4);
    }

    fn grow<B: GrowableBackend>(backend: &mut B) {
        backend.clear();
        let mut list = ListU32::create(backend).unwrap();

        for i in (0..10_000).step_by(13) {
            list.grow_for(i).unwrap();

            assert!(list.free() >= i);

            // And verify that no additional growing was initiated by .extend()
            let cap = list.capacity();
            list.extend(0..i as u32);
            assert_eq!(list.capacity(), cap);
        }
    }
}

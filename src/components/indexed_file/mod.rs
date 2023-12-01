pub mod entry;
pub mod iter;

use crate::backend::base::sub::BaseSubBackend;
use crate::backend::full::FullBackend;
use crate::backend::growable::GrowableBackend;
use crate::backend::Backend;
use crate::components::indexed_file::entry::Entry;
use crate::components::indexed_file::iter::IndexedFileIter;
use crate::components::split_file;
use crate::components::split_file::backend_index::BackendIndex;
use crate::components::split_file::SplitFile;
use crate::deser::{deserialize_impl, serialize_impl};
use crate::error::Error;
use crate::header::BaseHeader;
use crate::traits::creatable::Creatable;
use crate::traits::initiable::Initiable;
use crate::traits::mtype::MType;
use crate::Result;
use mult_split::MultiSplit;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::ops::Range;

/// A simple file storage/list where each inserted item gets its ID (incrementing number) which can
/// be used to retrieve the entry later on. It automatically regrows the underlying backend when
/// inserted data doesn't fit into the map thus the Backend needs to implement `GrowableBackend`.
pub struct IndexedFile<B> {
    backend: SplitFile<B>,
    count: usize,
}

impl<'b, B> IndexedFile<B>
    where
        B: Backend + 'b,
{
    /// Gets a deserializeable value from the IndexedFile by its ID.
    #[inline]
    pub fn get_t<T: DeserializeOwned>(&self, index: usize) -> Result<T> {
        deserialize_impl(self.get(index)?)
    }

    /// Gets the data of an entry by its ID.
    pub fn get(&self, id: usize) -> Result<&[u8]> {
        let index = self.entry_index(id)?;
        Ok(&self.backend.backend_data(BackendIndex::Second)[index])
    }

    /// Gets the entry by its ID as `FullBackend`.
    pub fn get_backend(&self, id: usize) -> Result<FullBackend<&[u8]>> {
        let index = self.entry_index(id)?;
        let data = &self.backend.backend_data(BackendIndex::Second)[index];
        Ok(FullBackend::new(data))
    }

    /// Gets the mutable data of an entry by its ID.
    pub fn get_mut(&mut self, id: usize) -> Result<&mut [u8]> {
        let index = self.entry_index(id)?;
        Ok(&mut self.backend.backend_data_mut(BackendIndex::Second)[index])
    }

    /// Gets the entry by its ID as `FullBackend` mutable.
    pub fn get_backend_mut(&mut self, id: usize) -> Result<FullBackend<Entry<B>>> {
        Ok(FullBackend::new(self.entry(id)?))
    }

    /// Gets multiple backends mutable at the same time.
    pub fn get_n<const N: usize>(&mut self, start_index: usize) -> Option<[&mut [u8]; N]> {
        if N == 0 || N + start_index > self.count() {
            return None;
        }

        let ranges: [Range<usize>; N] = std::array::from_fn(|i| {
            self.entry_index(i + start_index).unwrap() // We checked for oob before!
        });

        Some(self.get_n_by_ranges_unchecked(ranges))
    }

    /// Gets n items by their indices mutable at the same time.
    pub fn get_n_by_index<const N: usize>(
        &mut self,
        indices: [usize; N],
    ) -> Option<[&mut [u8]; N]> {
        if indices.iter().any(|i| !self.has_id(*i)) {
            return None;
        }
        let ranges: [Range<usize>; N] = indices.map(|i| {
            self.entry_index(i).unwrap() // We checked for oob before!
        });
        Some(self.get_n_by_ranges_unchecked(ranges))
    }

    #[inline]
    pub(crate) fn get_n_by_ranges_unchecked<const N: usize>(
        &mut self,
        ranges: [Range<usize>; N],
    ) -> [&mut [u8]; N] {
        let mut mult_split = MultiSplit::new(self.backend.backend_data_mut(BackendIndex::Second));
        ranges.map(|range| mult_split.borrow_mut(range).unwrap())
    }

    /// Gets two items in the `IndexedFile` mutable.
    pub fn get_two_mut(&mut self, first: usize, second: usize) -> Result<(&mut [u8], &mut [u8])> {
        let bes = self
            .get_n_by_index([first, second])
            .ok_or(Error::OutOfBounds)?;
        Ok(bes.into())
    }

    /// Gets an entry by its id.
    #[inline]
    pub fn entry(&mut self, id: usize) -> Result<Entry<B>> {
        let index = self.entry_index(id)?;
        Ok(Entry::new(self, id, index))
    }

    /// Flushes all data in the indexed file.
    #[inline]
    pub fn flush(&mut self) -> Result<()> {
        self.backend.flush()
    }

    /// Flushes a given item by its ID.
    pub fn flush_item(&mut self, id: usize) -> Result<()> {
        self.backend
            .flush_backend_range(BackendIndex::First, id * 8, 8)?;
        let range = self.entry_index(id)?;
        self.backend
            .flush_backend_range(BackendIndex::Second, range.start, range.len())?;
        Ok(())
    }

    /// Returns the amount of items stored in the IndexedFile.
    #[inline]
    pub fn count(&self) -> usize {
        self.count
    }

    /// Returns `true` if the IndexedFile is empty which is the case if no item has been pushed
    /// to the IndexedFile.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    #[inline]
    pub fn iter(&self) -> IndexedFileIter<B> {
        IndexedFileIter::new(self)
    }

    /// Returns `true` if the IndexedFile has an item for the given ID.
    #[inline]
    pub fn has_id(&self, id: usize) -> bool {
        (self.first().len() / 8) > id
    }

    /// Clears all data in the `IndexedFile`.
    pub fn clear(&mut self) {
        self.first_mut().clear();
        self.second_mut().clear();
        self.count = 0;
    }

    /*    /// Swaps two entries by their IDs.
        pub fn swap(&mut self, a: usize, b: usize) -> Result<()> {
            to do
            Ok(())
        }
    */

    /// Returns the index of the entry in self.second() by its ID.
    pub fn entry_index(&self, id: usize) -> Result<Range<usize>> {
        let offset = self.id_to_storage_offset(id)?;

        let next = if id + 1 == self.count {
            self.second().len()
        } else {
            self.id_to_storage_offset(id + 1)?
        };

        let header_len = BaseHeader::len_bytes();
        Ok((offset + header_len)..(next + header_len))
    }

    /// Gets the offset in self.second() of an entry given by its ID.
    #[inline]
    fn id_to_storage_offset(&self, id: usize) -> Result<usize> {
        let index = id * 8; // Numbers in the offset index are always 8 bytes (u32)
        let data: [u8; 8] = self.first().get(index, 8)?.try_into().unwrap();
        Ok(usize::from_le_bytes(data))
    }

    /// Gets the offset in self.second() of an entry given by its ID.
    #[inline]
    fn set_id_to_storage_offset(&mut self, id: usize, new_val: usize) -> Result<()> {
        let data = new_val.to_le_bytes();
        let mut fm = self.first_mut();
        let index = fm.get_index(id * 8);
        fm.data_mut()[index..index + 8].copy_from_slice(&data);
        Ok(())
    }

    #[inline]
    fn first(&self) -> BaseSubBackend<&[u8]> {
        self.backend.first()
    }

    #[inline]
    fn second(&self) -> BaseSubBackend<&[u8]> {
        self.backend.second()
    }

    #[inline]
    // fn first_mut(&mut self) -> BaseSubMutBackend<&mut [u8]> {
    fn first_mut(&mut self) -> split_file::entry::Entry<B> {
        self.backend.first_mut()
    }

    #[inline]
    fn second_mut(&mut self) -> split_file::entry::Entry<B> {
        self.backend.second_mut()
    }

    /// Returns `true` if the second list can fit the given amount of bytes.
    #[inline]
    fn second_can_fit(&self, bytes: usize) -> bool {
        self.second().free() >= bytes
    }

    /// Returns `true` if the first list can fit `item_count` items/indices.
    #[inline]
    fn first_can_fit(&self, item_count: usize) -> bool {
        self.first().free() >= item_count * 8
    }

    /// Shifts all offsets by a given offset `by` after a given id. `after_id` itself
    /// doesn't get updated.
    pub(crate) fn shift_offsets(&mut self, after_id: usize, by: isize) -> Result<bool> {
        let first_id = after_id + 1;
        if first_id >= self.count || by == 0 {
            return Ok(false);
        }

        // Shift first element and return an error on underflow. Since next id's will all be bigger
        // (or equal) to the first ID, we can assure that they won't underflow if the first one succeeds.
        let pos = self.id_to_storage_offset(first_id)?;
        let Some(res) = pos.checked_add_signed(by) else {
            return Err(Error::InvalidShift);
        };

        // Check that we don't shift over the current value.
        if self.id_to_storage_offset(after_id)? > res {
            return Err(Error::OutOfBounds);
        }

        self.set_id_to_storage_offset(first_id, res)?;

        // Iterate over the remaining IDs after first_id (if they exist) and update their values too.
        for id in (first_id + 1)..self.count {
            let pos = self.id_to_storage_offset(id)?;
            let res = pos
                .checked_add_signed(by)
                .expect("Tried to shift out of bounds");
            self.set_id_to_storage_offset(id, res)?;
        }

        Ok(true)
    }


    /// Shifts all offsets by a given offset `by` after a given id. `after_id` itself
    /// doesn't get updated.
    #[allow(dead_code)]
    pub(crate) fn partial_shift_offsets_unchecked(&mut self, after_id: usize, to_id: usize, by: usize) -> Result<bool> {
        let first_id = after_id + 1;
        /*
        if first_id >= self.count || by == 0 {
            return Ok(false);
        }
         */

        // Shift first element and return an error on underflow. Since next id's will all be bigger
        // (or equal) to the first ID, we can assure that they won't underflow if the first one succeeds.
        let pos = self.id_to_storage_offset(first_id)?;
        let Some(res) = pos.checked_add(by) else {
            return Err(Error::InvalidShift);
        };

        self.set_id_to_storage_offset(first_id, res)?;

        // Iterate over the remaining IDs after first_id (if they exist) and update their values too.
        for id in (first_id + 1)..to_id {
            let pos = self.id_to_storage_offset(id)?;
            let res = pos
                .checked_add(by)
                .expect("Tried to shift out of bounds");
            self.set_id_to_storage_offset(id, res)?;
        }

        Ok(true)
    }

    /// Shifts multiple elements using an iterator with a tuple in form of (after_id, by). The iterator has to yield
    /// the entry ids in ascending order!
    pub(crate) fn shift_multiple_offsets<I>(&mut self, shifts: I) -> Result<()>
        where
            I: IntoIterator<Item=(usize, isize)>,
    {
        let mut iter = shifts.into_iter().peekable();

        // Assert that the first item that gets peeked has a valid offset
        loop {
            let Some(peeked) = iter.peek() else {
                return Ok(());
            };
            if peeked.1 != 0 {
                break;
            }
            if iter.next().is_none() {
                return Ok(());
            }
        }

        // We can unwrap as the loop before would return if next is `None`;
        let mut current = iter.next().unwrap();
        let mut offset = 0;

        loop {
            let c_first_item = current.0 + 1;

            if c_first_item >= self.count {
                break;
            }

            let next = match iter.find(|i| i.1 != 0) {
                Some(v) => v,
                None => (self.count, 0),
            };

            let n_first_item: usize = next.0 + 1;

            // Current offset
            let c_offset = current.1 + offset;
            // ID to iterate to
            let end = n_first_item.min(self.count);
            for i in c_first_item..end {
                let pos = self.id_to_storage_offset(i)?;
                let Some(res) = pos.checked_add_signed(c_offset) else {
                    return Err(Error::InvalidShift);
                };
                self.set_id_to_storage_offset(i, res)?;
            }

            offset = c_offset;
            current = next;
        }

        Ok(())
    }

    #[inline]
    fn from_split_file(backend: SplitFile<B>) -> Result<Self> {
        let count = backend.first().len() / 8;
        Ok(Self { backend, count })
    }

    /*    fn header(&self) -> Vec<usize> {
            (0..self.count)
                .map(|i| self.id_to_storage_offset(i).unwrap())
                .collect()
        }
    */
}

impl<B> Creatable<B> for IndexedFile<B>
    where
        B: GrowableBackend,
{
    #[inline]
    fn with_capacity(backend: B, capacity: usize) -> Result<Self> {
        Self::from_split_file(SplitFile::create_with_init_cap(backend, capacity)?)
    }
}

impl<B> Initiable<B> for IndexedFile<B>
    where
        B: Backend,
{
    #[inline]
    fn init(backend: B) -> Result<Self> {
        Self::from_split_file(SplitFile::init(backend)?)
    }
}

impl<B> IndexedFile<B>
    where
        B: GrowableBackend,
{
    /// Inserts a serializeable value into the IndexedFile and returns its ID.
    pub fn insert_t<T: Serialize>(&mut self, item: &T) -> Result<usize> {
        let data = serialize_impl(item)?;
        let e = self.insert(&data)?;
        Ok(e)
    }

    /// Inserts raw data into the IndexedFile and returns its ID.
    pub fn insert(&mut self, data: &[u8]) -> Result<usize> {
        let id = self.count();

        let pos = self.second().len();
        self.add_index(pos)?;

        // Grows data if nedeed.
        self.grow_data_for(data.len())?;

        self.second_mut().push(data)?;
        Ok(id)
    }

    pub fn push_n_empty(&mut self, n: usize) -> Result<usize> {
        let first_id = self.count();
        if n == 0 {
            return Ok(first_id);
        }

        if !self.first_can_fit(n) {
            self.grow_list_by(n * 8)?;
        }

        for _ in 0..n {
            self.add_index(self.second().len())?;
        }

        Ok(first_id)
    }

    /// Inserts multiple raw items at once. This preallocates the required size if needed to reduce
    /// the amount of allocations. Returns the ID of the first entry in `items` if it was not empty or else `None`
    pub fn insert_n<I: AsRef<[U]>, U: AsRef<[u8]>>(&mut self, items: I) -> Result<Option<usize>> {
        let items = items.as_ref();
        if items.is_empty() {
            return Ok(None);
        }

        // Preallocate first list if needed
        if !self.first_can_fit(items.len()) {
            self.grow_list_by(items.len() * 8)?;
        }

        // Preallocate second list if needed
        let data_len: usize = items.iter().map(|i| i.as_ref().len()).sum();
        if !self.second_can_fit(data_len) {
            self.grow_data_by(data_len)?;
        }

        let first_id = self.insert(items[0].as_ref())?;

        // Insert data
        for i in &items[1..] {
            let i = i.as_ref();
            self.insert(i)?;
        }

        Ok(Some(first_id))
    }

    /// Inserts a given element <before> a given element in the IndexedFile, shifting all ids
    /// after `pos` by 1.
    pub fn insert_at(&mut self, data: &[u8], pos: usize) -> Result<()> {
        if pos > self.count() {
            return Err(Error::OutOfBounds);
        } else if pos == self.count {
            self.insert(data)?;
            return Ok(());
        }

        self.grow_data_for(data.len())?;

        let insert_index = self.id_to_storage_offset(pos)?;
        self.second_mut().replace(insert_index, 0, data)?;
        self.add_index_at(pos, insert_index)?;
        self.shift_offsets(pos, data.len() as isize)?;
        Ok(())
    }

    /// Grows a single entry by the given size with `value`.
    pub fn grow_entry(&mut self, id: usize, size: usize, value: u8) -> Result<()> {
        if size == 0 {
            return Ok(());
        }

        self.backend.grow(BackendIndex::Second, size)?;

        let index = self.entry_index(id)?.end - self.second().first_index();
        self.second_mut().replace_fill(index, 0, value, size)?;

        self.shift_offsets(id, size as isize)?;
        Ok(())
    }

    /// Grows a single entry and inserts `data`.
    pub fn grow_entry_with_data(&mut self, id: usize, data: &[u8]) -> Result<()> {
        let size = data.len();
        if size == 0 {
            return Ok(());
        }

        self.backend.grow(BackendIndex::Second, size)?;

        let index = self.entry_index(id)?.end - self.second().first_index();
        self.second_mut().replace(index, 0, data)?;

        self.shift_offsets(id, size as isize)?;
        Ok(())
    }

    /// Grows multiple entries. The entry IDs in `data` need to be ascending order.
    pub fn grow_multiple_fast<D: AsRef<[u8]>>(&mut self, entries: &[(usize, D)]) -> Result<()> {
        if entries.is_empty() {
            return Ok(());
        }

        let last_item = self.entry_index(self.count - 1).unwrap();

        let header_offset = BaseHeader::len_bytes();

        let last_id = entries[entries.len() - 1].0;
        let add_size: usize = entries.iter().map(|i| i.1.as_ref().len()).sum();
        let total_len = self.second().len() + add_size;
        let in_byte_end_index = total_len + header_offset;

        self.backend.grow(BackendIndex::Second, add_size)?;
        self.second_mut().inc_len(add_size).unwrap();

        // assert_eq!(in_byte_end_index, self.second().last_index());

        let mut moved = 0;
        let mut moved_end_idx = last_item.end;

        if last_id + 1 < self.count {
            let start = self.entry_index(last_id + 1).unwrap().start;
            let len = last_item.end - start;
            let new_index = in_byte_end_index - len;
            self.second_mut().move_range_to(start, len, new_index).unwrap();
            moved += len;
            moved_end_idx = start;
        }

        for (id, data) in entries.iter().rev() {
            let mut curr_idx = self.entry_index(*id)?;
            if curr_idx.end > last_item.end {
                curr_idx.end = last_item.end;
            }

            if curr_idx.end < moved_end_idx {
                let len = moved_end_idx - curr_idx.end;
                let new_index = in_byte_end_index - moved - len;
                self.second_mut().move_range_to(curr_idx.end, len, new_index).unwrap();
                moved += len;
            }

            let data = data.as_ref();
            let len = curr_idx.len();
            let new_index = in_byte_end_index - moved - len - data.len();
            self.second_mut().move_range_to(curr_idx.start, len, new_index).unwrap();
            self.second_mut().replace_same_len_direct(new_index + len, data).unwrap();
            // TODO: set the new offset here!

            moved += data.len() + len;
            moved_end_idx = curr_idx.start;
        }

        // TODO: optimize this shifting function into the moving code above by creating a partial_shift that doesn't go to the end but only to a given index and call this every time an item moves.
        self.shift_multiple_offsets(entries.iter().map(|i| (i.0, i.1.as_ref().len() as isize))).unwrap();

        Ok(())
    }

    /// Grows multiple entries. The entry IDs in `data` need to be ascending order.
    pub fn grow_multiple<D: AsRef<[u8]>>(&mut self, entries: &[(usize, D)]) -> Result<()> {
        if entries.is_empty() {
            return Ok(());
        }

        let total_size: usize = entries.iter().map(|i| i.1.as_ref().len()).sum();
        self.backend.grow(BackendIndex::Second, total_size)?;

        let mut offset = 0;
        for (id, data) in entries {
            let data = data.as_ref();

            let index = (self.entry_index(*id)?.end + offset).min(self.second().len() + BaseHeader::len_bytes()) - self.second().first_index();
            self.second_mut().replace(index, 0, data)?;
            offset += data.len();
        }

        self.shift_multiple_offsets(entries.iter().map(|i| (i.0, i.1.as_ref().len() as isize)))?;
        Ok(())
    }

    pub fn shrink_to_fit(&mut self) -> Result<()> {
        self.backend.shrink_to_fit()
    }

    /// Shrinks the entries size by `delta` without checking the entries 'len' bounds which means that meaningful data
    /// in the entry might be truncated.
    pub(crate) fn shrink_entry_unchecked(&mut self, id: usize, delta: usize) -> Result<()> {
        let entry_index = self.entry_index(id)?;
        if delta > entry_index.len() {
            return Err(Error::OutOfBounds);
        }

        let end_index = (entry_index.end - delta) - BaseHeader::len_bytes();

        let mut storage = self.second_mut();
        storage.replace(end_index, delta, &[])?;
        self.shift_offsets(id, -(delta as isize))?;
        self.backend.shrink(BackendIndex::Second, delta)?;
        Ok(())
    }

    /// Grows the file so that `add_entries` more entries and `total_entry_len` of total entry data
    /// can be inserted without regrowing the indexed file.
    pub fn grow(&mut self, add_entries: usize, total_entry_len: usize) -> Result<()> {
        let entry_size = add_entries * 8;
        self.backend.grow_both(entry_size, total_entry_len)?;
        Ok(())
    }

    /// Adds a new entry to the index, providing its position in self.second(). Returns the ID of the new entry.
    fn add_index_at(&mut self, pos: usize, data_pos: usize) -> Result<()> {
        if self.first().free() < 8 {
            self.grow_list()?;
        }

        let idx = pos * 8;
        let bytes = data_pos.to_le_bytes();
        self.first_mut().replace(idx, 0, &bytes)?;
        self.count += 1;
        Ok(())
    }

    /// Adds a new entry to the index, providing its position in self.second(). Returns the ID of the new entry.
    fn add_index(&mut self, index: usize) -> Result<()> {
        if self.first().free() < 8 {
            self.grow_list()?;
        }

        let bytes = index.to_le_bytes();
        self.first_mut().push(&bytes)?;
        self.count += 1;
        Ok(())
    }

    /// Grows the ID-Index mapping area of the IndexedFile.
    fn grow_list(&mut self) -> Result<()> {
        // println!("Growing list");

        // Duplicate current size
        let first_cap = self.first().capacity().max(8);
        self.grow_list_by(first_cap)?;

        Ok(())
    }

    /// Grows the ID-Index mapping area of the IndexedFile.
    fn grow_list_by(&mut self, size: usize) -> Result<()> {
        self.backend.grow(BackendIndex::First, size)?;
        Ok(())
    }

    /// Grows the data storage so `data` can fit. Does nothing if `data` already fits and
    /// no reallocating is necessary.
    fn grow_data_for(&mut self, len: usize) -> Result<usize> {
        if self.second_can_fit(len) {
            return Ok(0);
        }

        // println!("Growing data");
        let size = len.max(self.second().capacity());
        self.grow_data_by(size)?;
        Ok(size)
    }

    /// Grows the data storage area of the IndexedFile.
    fn grow_data_by(&mut self, size: usize) -> Result<()> {
        self.backend.grow(BackendIndex::Second, size)?;
        Ok(())
    }
}

impl<B, T> Extend<T> for IndexedFile<B>
    where
        B: GrowableBackend,
        T: Serialize,
{
    fn extend<I: IntoIterator<Item=T>>(&mut self, iter: I) {
        let iter: I::IntoIter = iter.into_iter();

        let (size, _) = iter.size_hint();
        if size > 0 && !self.first_can_fit(size * 8) {
            self.grow_list_by(size * 8).expect("Failed extending");
        }

        for i in iter {
            self.insert_t(&i).expect("Failed serializing extend item");
        }
    }
}

impl<B> MType for IndexedFile<B>
    where
        B: Backend,
{
    #[inline]
    fn raw_data(&self) -> &[u8] {
        self.backend.raw_data()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::backend::memory::test::{make_deeta, make_mem_backend};
    use crate::backend::mmap_mut::test::make_mmap_backend;
    use crate::components::indexed_file::entry::test::{
        check_test_data, insert_test_data, TEST_DATA_1, TEST_DATA_2, TEST_DATA_3, TEST_DATA_4,
    };
    use crate::traits::creatable::MemCreatable;

    #[test]
    fn test_all() {
        let mut backend = make_mem_backend(100);
        let mut indf = IndexedFile::create(&mut backend).unwrap();
        test_clear(&mut indf);
        test_shrink(&mut indf);
        test_push(&mut indf);
        test_grow(&mut indf);
        test_insert_at(&mut indf);
        test_two_mut(&mut indf);
        test_full_be(&mut indf);
        test_init(&mut backend);

        let mut backend = make_mmap_backend("stest2", 100);
        let mut indf = IndexedFile::create(&mut backend).unwrap();
        test_clear(&mut indf);
        test_shrink(&mut indf);
        test_push(&mut indf);
        test_grow(&mut indf);
        test_insert_at(&mut indf);
        test_two_mut(&mut indf);
        test_full_be(&mut indf);
        test_init(&mut backend);
    }

    fn test_init<B: GrowableBackend>(mut backend: &mut B) {
        backend.clear();

        let mut indf = IndexedFile::create(&mut backend).unwrap();
        indf.insert_t(&"hallo").unwrap();
        assert_eq!(indf.get_t(0), Ok("hallo".to_string()));
        assert_eq!(
            indf.get(0).map(|i| i.to_vec()),
            indf.get_mut(0).map(|i| i.to_vec())
        );

        let indf = IndexedFile::init(&mut backend).unwrap();
        assert_eq!(indf.get_t(0), Ok("hallo".to_string()));
    }

    fn test_push<B: GrowableBackend>(backend: &mut IndexedFile<B>) {
        backend.clear();

        let id = backend.insert_t(&"hallo").unwrap();
        assert_eq!(backend.get_t(0), Ok("hallo".to_string()));
        assert_eq!(id, 0);

        let id = backend.insert_t(&"other text").unwrap();
        assert_eq!(backend.get_t(1), Ok("other text".to_string()));
        assert_eq!(id, 1);

        backend.insert_t(&"fill me").unwrap();
        assert_eq!(backend.get_t(2), Ok("fill me".to_string()));
    }

    fn test_clear<B: GrowableBackend>(backend: &mut IndexedFile<B>) {
        assert!(backend.is_empty());

        backend.insert_t(&"hallo").unwrap();
        assert_eq!(backend.get_t(0), Ok("hallo".to_string()));
        assert_eq!(backend.count(), 1);

        backend.clear();

        assert_eq!(backend.count(), 0);
        assert_eq!(backend.get(0), Err(Error::OutOfBounds));
    }

    // #[test]
    #[allow(dead_code)]
    fn test_bug() {
        let mut backend = make_mem_backend(100);
        let mut indf = IndexedFile::create(&mut backend).unwrap();

        for i in make_deeta().take(10_000) {
            let enc = serialize_impl(&i).unwrap();
            indf.insert(&enc).unwrap();
        }
    }

    fn test_shrink<B: GrowableBackend>(backend: &mut IndexedFile<B>) {
        backend.clear();
        insert_test_data(backend);

        backend.shrink_entry_unchecked(0, 1).unwrap();
        assert_eq!(backend.get(0), Ok(&TEST_DATA_1[..TEST_DATA_1.len() - 1]));
        check_test_data(&backend, 1);

        backend.shrink_entry_unchecked(3, 1).unwrap();
        assert_eq!(backend.get(3), Ok(&TEST_DATA_4[..TEST_DATA_4.len() - 1]));

        backend.shrink_entry_unchecked(2, 2).unwrap();
        assert_eq!(backend.get(2), Ok(&TEST_DATA_3[..TEST_DATA_3.len() - 2]));

        backend
            .shrink_entry_unchecked(3, backend.get(3).unwrap().len())
            .unwrap();
        assert_eq!(backend.get(3), Ok(&[][..]));

        assert_eq!(
            backend.shrink_entry_unchecked(3, 1),
            Err(Error::OutOfBounds)
        )
    }

    fn test_grow<B: GrowableBackend>(backend: &mut IndexedFile<B>) {
        backend.clear();

        backend.insert(&[1, 2, 3]).unwrap();
        backend.insert(&[9, 9, 9]).unwrap();
        assert_eq!(backend.get(0), Ok(&[1, 2, 3][..]));
        assert_eq!(backend.get(1), Ok(&[9, 9, 9][..]));

        backend.grow_entry(0, 2, 0).unwrap();

        assert_eq!(backend.get(0), Ok(&[1, 2, 3, 0, 0][..]));
        assert_eq!(backend.get(1), Ok(&[9, 9, 9][..]));

        backend.grow_entry(1, 4, 3).unwrap();
        assert_eq!(backend.get(1), Ok(&[9, 9, 9, 3, 3, 3, 3][..]));

        backend.insert(&[5, 5]).unwrap();
        assert_eq!(backend.get(2), Ok(&[5, 5][..]));
        backend.grow_entry(2, 0, 0).unwrap();
        assert_eq!(backend.get(2), Ok(&[5, 5][..]));

        backend.clear();

        let mut backend = IndexedFile::create_mem_with_capacity(0).unwrap();
        backend.insert(&[1, 2, 3]).unwrap();
        backend.insert(&[3, 3, 8, 8]).unwrap();
        backend.insert(&[9, 9, 9]).unwrap();
        backend.insert(&[12, 12, 13]).unwrap();
        assert_eq!(backend.get(0), Ok(&[1, 2, 3][..]));
        assert_eq!(backend.get(2), Ok(&[9, 9, 9][..]));
        assert_eq!(backend.get(3), Ok(&[12, 12, 13][..]));

        let shifts: &[(usize, &[u8])] = &[(0, &[7, 9][..]), (2, &[1, 1, 1])][..];
        backend.grow_multiple_fast(shifts).unwrap();
        assert_eq!(backend.get(0), Ok(&[1, 2, 3, 7, 9][..]));
        assert_eq!(backend.get(2), Ok(&[9, 9, 9, 1, 1, 1][..]));
        assert_eq!(backend.get(3), Ok(&[12, 12, 13][..]));
    }

    #[test]
    fn test_grow_bug() {
        let mut file = IndexedFile::create_mem_with_capacity(10).unwrap();
        file.push_n_empty(1).unwrap();
        file.push_n_empty(1).unwrap();
        file.grow_multiple_fast(&[(0, &[0][..]), (1, &[0])][..]).unwrap();
        assert_eq!(file.get(0), Ok(&[0][..]));
        assert_eq!(file.get(1), Ok(&[0][..]));
    }

    fn test_full_be<B: GrowableBackend>(ifile: &mut IndexedFile<B>) {
        ifile.clear();
        insert_test_data(ifile);

        let be0 = ifile.get_backend(0).unwrap();
        assert_eq!(be0.data(), TEST_DATA_1);
        assert_eq!(be0.len(), TEST_DATA_1.len());

        let be1 = ifile.get_backend(1).unwrap();
        assert_eq!(be1.data(), TEST_DATA_2);
        assert_eq!(be1.len(), TEST_DATA_2.len());

        let mut be0 = ifile.get_backend_mut(0).unwrap();
        be0.grow(1).unwrap();
        let mut exp = TEST_DATA_1.to_vec();
        exp.push(0);
        assert_eq!(be0.data(), &exp);
        assert_eq!(be0.len(), exp.len());

        let be1 = ifile.get_backend(1).unwrap();
        assert_eq!(be1.data(), TEST_DATA_2);
        assert_eq!(be1.len(), TEST_DATA_2.len());
    }

    fn test_two_mut<B: GrowableBackend>(backend: &mut IndexedFile<B>) {
        backend.clear();
        insert_test_data(backend);

        let (f, s) = backend.get_two_mut(2, 0).unwrap();
        assert_eq!(f, TEST_DATA_3);
        assert_eq!(s, TEST_DATA_1);

        let (f, s) = backend.get_two_mut(3, 1).unwrap();
        assert_eq!(f, TEST_DATA_4);
        assert_eq!(s, TEST_DATA_2);

        let (f, s) = backend.get_two_mut(3, 0).unwrap();
        assert_eq!(f, TEST_DATA_4);
        assert_eq!(s, TEST_DATA_1);

        let (f, s) = backend.get_two_mut(0, 2).unwrap();
        assert_eq!(f, TEST_DATA_1);
        assert_eq!(s, TEST_DATA_3);

        let (f, s) = backend.get_two_mut(1, 3).unwrap();
        assert_eq!(f, TEST_DATA_2);
        assert_eq!(s, TEST_DATA_4);

        let (f, s) = backend.get_two_mut(0, 3).unwrap();
        assert_eq!(f, TEST_DATA_1);
        assert_eq!(s, TEST_DATA_4);

        let r = backend.get_two_mut(0, 4);
        assert_eq!(r, Err(Error::OutOfBounds));
    }

    fn test_insert_at<B: GrowableBackend>(backend: &mut IndexedFile<B>) {
        backend.clear();
        backend.insert(&[0u8; 40]).unwrap();
        backend.clear();

        backend.insert(TEST_DATA_1).unwrap();
        backend.insert(TEST_DATA_2).unwrap();
        backend.insert(TEST_DATA_3).unwrap();

        assert_eq!(backend.get(0), Ok(TEST_DATA_1));
        assert_eq!(backend.get(1), Ok(TEST_DATA_2));
        assert_eq!(backend.get(2), Ok(TEST_DATA_3));

        backend.insert_at(TEST_DATA_4, 1).unwrap();
        assert_eq!(backend.get(0), Ok(TEST_DATA_1));
        assert_eq!(backend.get(1), Ok(TEST_DATA_4));
        assert_eq!(backend.get(2), Ok(TEST_DATA_2));
        assert_eq!(backend.get(3), Ok(TEST_DATA_3));

        backend.insert_at(TEST_DATA_1, 3).unwrap();
        assert_eq!(backend.get(0), Ok(TEST_DATA_1));
        assert_eq!(backend.get(1), Ok(TEST_DATA_4));
        assert_eq!(backend.get(2), Ok(TEST_DATA_2));
        assert_eq!(backend.get(3), Ok(TEST_DATA_1));
        assert_eq!(backend.get(4), Ok(TEST_DATA_3));
    }

    #[allow(dead_code)]
    fn test_big() {
        let backend = make_mmap_backend("bigfile", 1024);
        let mut indf = IndexedFile::create(backend).unwrap();

        let mut bytes = 0;
        for i in (10..)
            .step_by(13)
            .map(|i| format!("{i}_{i}").repeat(i % 30))
        {
            if bytes > 5 * 1024 * 1024 * 1024 {
                break;
            }

            indf.insert(i.as_bytes()).unwrap();

            bytes += i.as_bytes().len() + 10;
        }
    }
}

pub mod hashing;
pub mod insertion;
mod kvpair;
mod metadata;
mod primes;
pub mod iter;

use crate::backend::base::sub::BaseSubBackend;
use crate::backend::base::sub_mut::GeneralSubMutBackend;
use crate::backend::growable::GrowableBackend;
use crate::backend::Backend;
use crate::components::indexed_file::IndexedFile;
use crate::components::list::ListU32;
use crate::components::map::hashing::hashfn::{DoubleHashing, HashFn, LinearProbing, QuadraticProbing};
use crate::components::map::insertion::Insertion;
use crate::components::map::kvpair::KVPair;
use crate::components::map::metadata::MapMetadata;
use crate::components::multi_file::entry_mut::MFileEntryMut;
use crate::components::multi_file::MultiFile;
use crate::traits::creatable::Creatable;
use crate::traits::deser::Deser;
use crate::traits::initiable::Initiable;
use crate::utils::smallest_two_power_for;
use crate::{Error, Result};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::cmp::Ordering;
use std::marker::PhantomData;
use crate::components::map::iter::MapIter;
use crate::traits::mtype::MType;

// TODO: [maybe] Convert to sorted list and allow binary search. Simple since the array used currently as hashtable can be used as sorted position, allowing binary search.

/// Max amount of entries that can be inserted until the map regrows in percent.
const MAX_LOAD: f32 = 0.75;

// Max amount of entries that can be inserted until the map regrows in percent.
// const MIN_LOAD: f32 = 0.3;

/// The default hashing algorithm
// type DefaultHasher = QuadraticProbing;
type DefaultHasher = DoubleHashing<QuadraticProbing, LinearProbing>;

/// A HashMap similar data structure working entirely stored in the given backend. Supports growing if it gets too full
pub struct FMap<B, K, V, H = DefaultHasher> {
    pub backend: MultiFile<B>,

    /// The amount of items in the map.
    len: usize,

    /// The amount of items that can be stored in the map without rehashing.
    capacity: usize,

    p: PhantomData<(K, V, H)>,
}

impl<B, K, V, H> FMap<B, K, V, H>
    where
        H: HashFn,
        B: GrowableBackend,
        K: hashing::Hash + Eq + Deser,
        V: Deser,
{
    /// Inserts a new key value pair into the map returning its unique, non changing ID. If the key already existed, the
    /// value *DOES NOT* get updated.
    #[inline]
    pub fn insert(&mut self, k: &K, v: &V) -> Result<u32> {
        self.insert_debug(k, v).map(|i| i.kv_id())
    }

    /// Inserts a new key value pair into the map returning its unique, non changing ID. If the key already existed, the
    /// value *DOES NOT* get updated.
    pub fn insert_debug(&mut self, k: &K, v: &V) -> Result<Insertion> {
        if self.need_grow() {
            self.grow()?;
        }

        let res = self.raw_insert(k, v)?;

        // Increment length if was newly inserted
        if res.inserted() {
            self.inc_len(1)?;
        }

        Ok(res)
    }

    /// Inserts the key value pair into the map assuming there is enough space and the insertion won't break the
    /// load factor invariant! It also won't increase the maps length counter.
    fn raw_insert(&mut self, k: &K, v: &V) -> Result<Insertion> {
        let pair = KVPair::new(k, v);
        let pair_id = self.insert_entry(&pair)?;
        self.map_kv_pair_s(k, pair_id)
    }

    fn map_kv_pair_s(&mut self, key: &K, kv_id: u32) -> Result<Insertion> {
        let capacity = self.capacity();
        let (mut table, mut kv_storage) = self.kv_and_table_mut()?;
        Self::map_kv_pair(key, kv_id, capacity, &mut table, &mut kv_storage)
    }

    /// Maps a KV pair to its hash.
    fn map_kv_pair<E1: Backend, E2: Backend>(
        key: &K,
        kv_id: u32,
        capacity: usize,
        table: &mut ListU32<E1>,
        kv_storage: &mut IndexedFile<E2>,
    ) -> Result<Insertion> {
        let key_hash = key.hash();

        for i in 0..capacity {
            let hash = H::f(key_hash, i, capacity);

            if let Some(pair_id) = Self::resolve_hash(hash, table) {
                let data = kv_storage.get(pair_id as usize)?;
                let kv_pair: KVPair<K, V> = bincode::deserialize(data)?;

                if kv_pair.key() == key {
                    // KV pair already exists in the map and key is the same (so not just a collision)
                    return Ok(Insertion::new(pair_id, i, false, hash as usize));
                }

                // Continue as we haven't found the key yet.
                continue;
            }

            // Insert the key into the current free slot
            Self::set_table_kvid(hash as usize, kv_id, table)?;
            return Ok(Insertion::new(kv_id, i, true, hash as usize));
        }

        // The hash function must be implemented in a way that it doesn't need more than `capacity` iterations to find a free slot.
        unreachable!()
    }

    /// Grows the Map to the next prime of nth power of 2 so that the load factor for the given length is <= `MAX_LOAD`.
    /// This rehashes all entries and should only be called if the load factor is getting bigger than the
    /// given limit (but hasn't exceeded this limit yet).
    pub fn grow_to(&mut self, len: usize) -> Result<usize> {
        // Increase capacity to the next prime of amount of new items we want to add.
        let need_cap = (len as f32 / MAX_LOAD).ceil() as usize;
        let pow = smallest_two_power_for(need_cap) as usize;
        let regrowth_size = primes::NEXT_PRIMES_OF_TWO[pow] as usize;
        if self.capacity() == regrowth_size {
            return Ok(0);
        }

        self.increase_capacity(regrowth_size)?;

        // Rehash all exitsing entries
        self.rehash()?;

        Ok(regrowth_size)
    }

    pub fn reserve_storage(&mut self, items: usize, bytes: usize) -> Result<()> {
        self.kv_storage_mut().grow(items, bytes)?;
        Ok(())
    }

    /// Increases the capacity, leaving the hash table in an invalid state as the hashing function isn't valid anymore.
    /// It also clears the hash table. This means you have to rehash all entries again!
    fn increase_capacity(&mut self, new_capacity: usize) -> Result<()> {
        if new_capacity <= self.capacity {
            return Ok(());
        }
        let diff = new_capacity - self.capacity;

        let mut table = self.table_list_mut();
        table.grow_for_exact(diff)?;
        table.set_len(new_capacity)?;
        table.mem_set(0)?;

        self.set_capacity(new_capacity)?;
        Ok(())
    }

    /// Hashes all elements in the KV store again.
    pub fn rehash(&mut self) -> Result<()> {
        let len = self.len();
        let capacity = self.capacity();

        let (mut table, mut kv_storage) = self.kv_and_table_mut()?;

        for i in 0..len as u32 {
            let kv = Self::entry_by_id(i, &kv_storage).unwrap();
            Self::map_kv_pair(kv.key(), i, capacity, &mut table, &mut kv_storage)?;
        }

        Ok(())
    }

    /// Grows the Map to the next prime of nth power of 2 so that the load factor is <= `MAX_LOAD` for a single new entry.
    /// This rehashes all entries and should only be called if the load factor is getting bigger than the
    /// given limit (but hasn't exceeded this limit yet).
    fn grow(&mut self) -> Result<usize> {
        self.grow_to(self.len() + 1)
    }
}

impl<B, K, V, H> FMap<B, K, V, H>
    where
        H: HashFn,
        B: Backend,
        K: hashing::Hash + Eq + Deser,
        V: DeserializeOwned,
{
    #[inline]
    pub fn get(&self, k: &K) -> Option<V> {
        self.get_debug(k).map(|i| i.0)
    }

    pub(crate) fn get_debug(&self, k: &K) -> Option<(V, usize)> {
        let key_hash = k.hash();

        let table_list = self.hash_table();
        let kv_storage = self.entry_storage();

        for i in 0..self.capacity {
            let hash = H::f(key_hash, i, self.capacity);
            let kv_pair_id = Self::resolve_hash(hash, &table_list)?;
            let kv_item: KVPair<K, V> = Self::entry_by_id(kv_pair_id, &kv_storage).unwrap();
            if kv_item.key() == k {
                return Some((kv_item.into_value(), i));
            }
        }

        None
    }

    /// Hashes all elements in the map using a comparing function that prefers some items over other ones when a collision occurs.
    /// This means that, in average, the items that have a higher order (defined by the `compare` function) will have less to zero
    /// collisions making lookup for those faster and potentially other items with less relevance slower.
    pub fn rehash_with_relevance<R>(&mut self, mut compare: R) -> Result<()>
        where
            R: FnMut(&KVPair<K, V>, &KVPair<K, V>) -> Ordering,
    {
        self.clear_table()?;

        // We don't change the amount of entries or capacity here so they don't need to be reevaluated in the algorithm.
        let len = self.len();
        let capacity = self.capacity();

        let (mut table, kv_storage) = self.kv_and_table_mut()?;

        let mut mapped_entries = 0;

        // Iterate over all entries
        for mut entry_id in 0..len as u32 {
            // Current entry we want to find a position in the table for.
            let mut entry = Self::entry_by_id(entry_id, &kv_storage).unwrap();
            let mut key_hash = entry.key().hash();

            let mut i = 0;
            loop {
                let hash = H::f(key_hash, i, capacity);

                // Hashed position already occupied
                if let Some(pair_id) = Self::resolve_hash(hash, &table) {
                    let occupied: KVPair<K, V> =
                        bincode::deserialize(kv_storage.get(pair_id as usize)?)?;
                    assert!(occupied.key() != entry.key());

                    // If the entry we currently want to insert is not more important than the current found occupied entry
                    // keep searching for a free slot!
                    if compare(&entry, &occupied) != Ordering::Greater {
                        i += 1;
                        continue;
                    }

                    // Swap occupied entry with current entry if it has a higher relevance.
                    Self::set_table_kvid(hash as usize, entry_id, &mut table)?;
                    entry_id = pair_id;
                    key_hash = occupied.key().hash();
                    entry = occupied;

                    i = 0;
                    loop {
                        if H::f(key_hash, i, capacity) == hash {
                            break;
                        }

                        i += 1;
                    }
                } else {
                    mapped_entries += 1;
                    Self::set_table_kvid(hash as usize, entry_id, &mut table)?;
                    break;
                }
            }
        }

        assert_eq!(mapped_entries, len);

        Ok(())
    }
}

impl<B, K, V, H> FMap<B, K, V, H>
    where
        B: Backend,
        V: DeserializeOwned,
        K: DeserializeOwned,
{
    /// Returns the KV Pair for a given KV-Pair-ID
    #[inline]
    fn entry_by_id<E: Backend>(id: u32, kv_storage: &IndexedFile<E>) -> Option<KVPair<K, V>> {
        let raw = kv_storage.get(id as usize).ok()?;
        bincode::deserialize(raw).ok()
    }
}

impl<B, K, V, H> FMap<B, K, V, H>
    where
        B: GrowableBackend,
        V: Serialize,
        K: Serialize,
{
    /// Inserts a key with its value into the KV pair storage returning its ID.
    #[inline]
    fn insert_entry(&mut self, pair: &KVPair<&K, &V>) -> Result<u32> {
        let enc = bincode::serialize(pair)?;
        Ok(self.kv_storage_mut().insert(&enc)? as u32)
    }
}

impl<B, K, V, H> FMap<B, K, V, H>
    where
        B: Backend,
{
    #[inline]
    pub fn iter(&self) -> MapIter<B, K, V, H> {
        MapIter::new(self)
    }

    /// Returns the HashTable and KVStorage both mutable.
    fn kv_and_table_mut(
        &mut self,
    ) -> Result<(
        ListU32<GeneralSubMutBackend>,
        IndexedFile<GeneralSubMutBackend>,
    )> {
        let (header_be, kv_be) = self.backend.get_two_mut(1, 2)?;
        Ok((ListU32::init(header_be)?, IndexedFile::init(kv_be)?))
    }

    /// Returns the table list.
    #[inline]
    fn hash_table(&self) -> ListU32<BaseSubBackend<&[u8]>> {
        self.backend.get_backend(1).unwrap()
    }

    /// Returns the Key-Value-pair storage.
    #[inline]
    fn entry_storage(&self) -> IndexedFile<BaseSubBackend<&[u8]>> {
        self.backend.get_backend(2).unwrap()
    }

    /// Returns the KV storage ID for a given hash value.
    fn resolve_hash<E: Backend>(hash: u64, table_list: &ListU32<E>) -> Option<u32> {
        let e: [u8; 4] = table_list.get_raw(hash as usize).ok()?.try_into().unwrap();
        let e = u32::from_le_bytes(e);
        if e > 0 {
            Some(e - 1)
        } else {
            None
        }
    }

    /// Inserts the given `kv_id` into the hash table at the given position.
    #[inline]
    fn set_table_kvid<E: Backend>(pos: usize, kv_id: u32, table: &mut ListU32<E>) -> Result<()> {
        // Use low endian bytes here as this is much faster for integer and always size of N
        table.set_raw(pos, &(kv_id + 1).to_le_bytes())?;
        Ok(())
    }

    /// Clears all entries from the map.
    pub fn clear(&mut self) -> Result<()> {
        let (mut table, mut kv_storage) = self.kv_and_table_mut()?;

        Self::table_clear(&mut table)?;
        kv_storage.clear();

        self.len = 0;
        Ok(())
    }

    /// Flushes the whole map.
    #[inline]
    pub fn flush(&mut self) -> Result<()> {
        self.backend.flush()
    }

    /// Sets the maps metadata stored in the backend
    #[inline]
    fn set_metadata(&mut self, md: MapMetadata) -> Result<()> {
        let mut metadata_be = self.backend.get_mut(0).unwrap();
        metadata_be.replace_same_len(0, &md.to_bytes())?;
        Ok(())
    }

    /// Increments the maps length by `amount`.
    #[inline]
    fn inc_len(&mut self, amount: usize) -> Result<()> {
        self.set_metadata(MapMetadata::new(self.len + amount, self.capacity))?;
        self.len += amount;
        Ok(())
    }

    /// Sets the maps capacity to `capacity`
    #[inline]
    fn set_capacity(&mut self, capacity: usize) -> Result<()> {
        self.capacity = capacity;
        self.set_metadata(MapMetadata::new(self.len, capacity))
    }

    /// Clears the hash table.
    fn clear_table(&mut self) -> Result<()> {
        let (mut table, _) = self.kv_and_table_mut()?;
        Self::table_clear(&mut table)?;
        Ok(())
    }

    /// Clears the hash table by setting all values to 0 but keeping the capacity and length.
    #[inline]
    fn table_clear<E: Backend>(table: &mut ListU32<E>) -> Result<()> {
        table.mem_set(0)?;
        Ok(())
    }
}

impl<B, K, V, H> FMap<B, K, V, H>
    where
        B: GrowableBackend,
{
    /// Returns the table list mutable.
    #[inline]
    fn table_list_mut(&mut self) -> ListU32<MFileEntryMut<B>> {
        self.backend.get_backend_mut(1).unwrap()
    }

    /// Returns the Key-Value-pair storage mutable.
    #[inline]
    fn kv_storage_mut(&mut self) -> IndexedFile<MFileEntryMut<B>> {
        self.backend.get_backend_mut(2).unwrap()
    }

    /// Allocates more space to store n new entries with a total encoded size
    #[inline]
    pub fn preallocate_entries(&mut self, entry_count: usize, data_len: usize) -> Result<()> {
        self.kv_storage_mut().grow(entry_count, data_len)
    }
}

impl<B, K, V, H> FMap<B, K, V, H> {
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns `true` if the hashmap can fit additional new items without needing to grow.
    #[inline]
    pub fn can_fit(&self, additional: usize) -> bool {
        self.load_factor_for(self.len() + additional) < MAX_LOAD
    }

    /// Returns the Load factor of the Map.
    #[inline]
    pub fn load_factor(&self) -> f32 {
        self.load_factor_for(self.len())
    }

    /// Returns the Load factor for a given length in the Map.
    #[inline]
    fn load_factor_for(&self, len: usize) -> f32 {
        if self.capacity() == 0 {
            return 0.0;
        }

        len as f32 / self.capacity() as f32
    }

    /// Returns `true` if the map needs to grow and rehash for a new element.
    #[inline]
    fn need_grow(&self) -> bool {
        self.need_grow_for(1)
    }

    /// Returns `true` if the map needs to grow and rehash for n new elements.
    #[inline]
    fn need_grow_for(&self, new_elements: usize) -> bool {
        self.load_factor_for(self.len() + new_elements) >= MAX_LOAD
    }
}

impl<B, K, V, H> Creatable<B> for FMap<B, K, V, H>
    where
        B: GrowableBackend,
{
    fn with_capacity(backend: B, capacity: usize) -> Result<Self> {
        let cap = primes::next_bigger_than(capacity) as usize;

        let mut backend = MultiFile::with_capacity(backend, cap)?;

        let mut capacity_metadata = backend.insert_empty()?;
        capacity_metadata.grow_to(MapMetadata::byte_len())?;
        capacity_metadata
            .push(&MapMetadata::new(0, cap).to_bytes())
            .unwrap();

        let mut table: ListU32<_> = backend.insert_new_backend()?;
        table.grow_for_exact(cap)?;
        table.set_len(cap)?;
        table.mem_set(0)?;

        let mut kv_storage: IndexedFile<_> = backend.insert_new_backend()?;
        kv_storage.grow(cap, cap)?;

        Ok(Self {
            backend,
            len: 0,
            capacity: cap,
            p: PhantomData,
        })
    }
}

impl<B, K, V, H> Initiable<B> for FMap<B, K, V, H>
    where
        B: Backend,
{
    fn init(backend: B) -> Result<Self> {
        let backend = MultiFile::init(backend)?;

        let metadata_be = backend.get(0).ok_or(Error::Initialization)?;
        let metadata = MapMetadata::from_bytes(metadata_be.get(0, MapMetadata::byte_len())?);

        Ok(Self {
            len: metadata.len(),
            capacity: metadata.capacity(),
            backend,
            p: PhantomData,
        })
    }
}

impl<B, K, V, H> Extend<(K, V)> for FMap<B, K, V, H>
    where
        H: HashFn,
        B: GrowableBackend,
        K: hashing::Hash + Eq + Deser,
        V: Deser,
{
    fn extend<T: IntoIterator<Item=(K, V)>>(&mut self, iter: T) {
        let iter = iter.into_iter();
        let (lower, upper) = iter.size_hint();

        let mut did_pregrow = false;
        if lower > 0 && Some(lower) == upper && self.need_grow_for(lower) {
            self.grow_to(self.len() + lower).expect("Failed to grow");
            did_pregrow = true;
        }

        if did_pregrow {
            let mut insert_count = 0;
            for (k, v) in iter {
                let res = self.raw_insert(&k, &v).expect("Failed to insert");
                if res.inserted() {
                    insert_count += 1;
                }
            }

            self.inc_len(insert_count)
                .expect("Failed to set maps length");
        } else {
            for (k, v) in iter {
                self.insert(&k, &v).expect("Failed to insert");
            }
        }
    }
}

impl<'a, B, K, V, H> Extend<(&'a K, &'a V)> for FMap<B, K, V, H>
    where
        H: HashFn,
        B: GrowableBackend,
        K: hashing::Hash + Eq + Deser,
        V: Deser,
{
    fn extend<T: IntoIterator<Item=(&'a K, &'a V)>>(&mut self, iter: T) {
        let iter = iter.into_iter();
        let (lower, upper) = iter.size_hint();

        let mut did_pregrow = false;
        if lower > 0 && Some(lower) == upper && self.need_grow_for(lower) {
            self.grow_to(self.len() + lower).expect("Failed to grow");
            did_pregrow = true;
        }

        if did_pregrow {
            let mut insert_count = 0;
            for (k, v) in iter {
                let res = self.raw_insert(k, v).expect("Failed to insert");
                if res.inserted() {
                    insert_count += 1;
                }
            }

            self.inc_len(insert_count)
                .expect("Failed to set maps length");
        } else {
            for (k, v) in iter {
                self.insert(k, v).expect("Failed to insert");
            }
        }
    }
}

impl<'a, B, K, V, H> Extend<&'a (K, V)> for FMap<B, K, V, H>
    where
        H: HashFn,
        B: GrowableBackend,
        K: hashing::Hash + Eq + Deser,
        V: Deser,
{
    #[inline]
    fn extend<T: IntoIterator<Item=&'a (K, V)>>(&mut self, iter: T) {
        self.extend(iter.into_iter().map(|i: &(K, V)| (&i.0, &i.1)));
    }
}

impl<B, K, V, H> MType for FMap<B, K, V, H> where B: Backend {
    #[inline]
    fn raw_data(&self) -> &[u8] {
        self.backend.raw_data()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::backend::memory::test::{make_deeta, make_mem_backend};
    use crate::backend::memory::{MemoryBackend, MemoryData};
    use crate::backend::mmap_mut::test::make_mmap_backend;
    use std::time::Instant;

    // #[test]
    #[allow(dead_code)]
    fn test_bug() {
        let be = MemoryBackend::from_storage(MemoryData::new(vec![0u8; 100_000])).unwrap();
        let mut map: FMap<_, String, u32> = FMap::with_capacity(be, 100_000).unwrap();
        map.extend(make_deeta().take(10_000).map(|i| (i, 32u32)));
    }

    #[test]
    fn test_all() {
        let mut mem_backend = make_mem_backend(100);
        test_grow(&mut mem_backend);
        test_create(&mut mem_backend);
        test_insert(&mut mem_backend);
        test_load(&mut mem_backend);
        test_not_contained(&mut mem_backend);
        test_rehash_with_relevance(&mut mem_backend);

        let mut mmap_backend = make_mmap_backend("./fmap", 100);
        test_grow(&mut mmap_backend);
        test_create(&mut mmap_backend);
        test_insert(&mut mmap_backend);
        test_load(&mut mmap_backend);
        test_not_contained(&mut mmap_backend);
    }

    fn test_grow<B: GrowableBackend>(mut backend: &mut B) {
        backend.clear();
        let mut map: FMap<_, String, u32> = FMap::with_capacity(&mut backend, 0).unwrap();
        assert_eq!(map.capacity(), 1);
        map.insert(&"hallo".to_string(), &99).unwrap();
        assert_eq!(map.capacity(), 3);
        map.insert(&"bye".to_string(), &19).unwrap();
        assert_eq!(map.capacity(), 3);
        map.insert(&"none".to_string(), &7).unwrap();
        assert_eq!(map.capacity(), 5);
        map.insert(&"some".to_string(), &33).unwrap();
        assert_eq!(map.capacity(), 11);
    }

    fn test_create<B: GrowableBackend>(mut backend: &mut B) {
        backend.clear();

        let mut map: FMap<_, String, usize> = FMap::with_capacity(&mut backend, 10_000).unwrap();

        let terms: Vec<_> = (0..100_000).map(|i| format!("{i}").repeat(5)).collect();
        let start = Instant::now();
        map.extend(terms.iter().cloned().enumerate().map(|i| (i.1, i.0)));
        println!(
            "Inserting {} items took {:?}; Grew map to: {}. New lf: {}",
            terms.len(),
            start.elapsed(),
            map.capacity,
            map.load_factor()
        );
        let start = Instant::now();
        map.flush().unwrap();
        println!("Flushing for {} elements took: {:?}", terms.len(), start.elapsed());

        for (pos, i) in terms.iter().enumerate() {
            assert_eq!(map.get(i), Some(pos));
        }

        assert_eq!(map.len, map.iter().count());
        let mut got: Vec<_> = map.iter().map(|i| i.0).collect();
        got.sort_unstable();
        let mut expect = terms.clone();
        expect.sort_unstable();
        assert_eq!(got, expect);
    }

    fn test_insert<B: GrowableBackend>(mut backend: &mut B) {
        backend.clear();
        let mut map: FMap<_, String, u32> = FMap::with_capacity(&mut backend, 0).unwrap();
        map.extend([
            ("a".to_string(), 1),
            ("b".to_string(), 10),
            ("a".to_string(), 2),
        ]);
        assert_eq!(map.len(), 2);
        assert_eq!(map.capacity(), 5);
        assert_eq!(map.get(&"a".to_string()), Some(1));
    }

    fn test_load<B: GrowableBackend>(mut backend: &mut B) {
        backend.clear();
        let mut map: FMap<_, String, u32> = FMap::with_capacity(&mut backend, 0).unwrap();
        let data: Vec<_> = make_deeta().take(100).map(|i| (i, 1235)).collect();
        map.extend(data.iter());
        let len = map.len();
        let cap = map.capacity();

        let map: FMap<_, String, u32> = FMap::init(backend).unwrap();
        for i in data.iter() {
            assert_eq!(map.get(&i.0), Some(i.1));
        }
        assert_eq!(map.len(), len);
        assert_eq!(map.capacity(), cap);
    }

    fn test_not_contained<B: GrowableBackend>(mut backend: &mut B) {
        backend.clear();
        let mut map: FMap<_, String, u32> = FMap::with_capacity(&mut backend, 0).unwrap();
        let data: Vec<_> = make_deeta().take(100).map(|i| (i, 1235)).collect();
        map.extend(data.iter());

        for i in make_deeta().skip(100).take(1000) {
            assert_eq!(map.get(&i), None);
        }
    }

    fn test_rehash_with_relevance<B: GrowableBackend>(mut backend: &mut B) {
        backend.clear();
        let mut map: FMap<_, String, u32> = FMap::with_capacity(&mut backend, 1031).unwrap();
        let end = 773;

        let data: Vec<_> = make_deeta()
            .take(end)
            .enumerate()
            .map(|i| (i.1, (i.0 % 13) as u32))
            .collect();

        map.extend(data.iter());

        // Sanity check
        let mut no_rehash_collisions = 0;
        for (i, v) in &data {
            let (val, collisions) = map.get_debug(&i.to_string()).unwrap();
            assert_eq!(val, *v);
            if collisions > 0 && *v == 7 {
                no_rehash_collisions += 1;
            }
        }
        // we need at laest one collision for an item with value==7 so we can ensure collisions have actually been reduced.
        assert!(no_rehash_collisions > 0);

        map.rehash_with_relevance(|a, b| {
            if *a.value() == 7 && *b.value() != 7 {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        })
            .unwrap();

        let mut rehash_collisions = 0;
        let mut rehash_collisions_tot = 0;
        for (i, v) in &data {
            let (val, collisions) = map.get_debug(&i.to_string()).unwrap();
            assert_eq!(val, *v);
            if collisions > 0 && *v == 7 {
                rehash_collisions += 1;
                rehash_collisions_tot += collisions;
            }
        }

        assert_eq!(rehash_collisions, 0);
        assert_eq!(rehash_collisions_tot, 0);
    }
}

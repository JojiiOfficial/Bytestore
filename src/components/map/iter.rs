use serde::de::DeserializeOwned;
use crate::backend::Backend;
use crate::backend::base::sub::BaseSubBackend;
use crate::components::indexed_file::IndexedFile;
use crate::components::list::ListU32;
use crate::components::map::FMap;

pub struct MapIter<'a, B, K, V, H> {
    // TODO: remove unused parameter and use phantom data!
    map: &'a FMap<B, K, V, H>,
    table: ListU32<BaseSubBackend<'a, &'a [u8]>>,
    storage: IndexedFile<BaseSubBackend<'a, &'a [u8]>>,
    pos: usize,
}

impl<'a, B, K, V, H> MapIter<'a, B, K, V, H> where B: Backend {
    #[inline]
    pub(super) fn new(map: &'a FMap<B, K, V, H>) -> Self {
        let table = map.hash_table();
        let storage = map.entry_storage();
        Self { map, pos: 0, table, storage }
    }
}

impl<'a, B, K, V, H> MapIter<'a, B, K, V, H> where B: Backend {
    #[inline]
    fn find_next_occupied(&mut self) -> Option<(usize, usize)> {
        (self.pos..self.table.len()).find_map(|index| {
            let item = self.table.get(index).ok().unwrap() as usize;
            if item > 0 {
                Some((item - 1, index))
            } else {
                None
            }
        })
    }
}

impl<'a, B, K, V, H> Iterator for MapIter<'a, B, K, V, H>
    where B: Backend,
          K: DeserializeOwned,
          V: DeserializeOwned,
{
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        let (kv_id, pos) = self.find_next_occupied()?;
        self.pos = pos + 1;
        Some(FMap::<B, K, V>::entry_by_id(kv_id as u32, &self.storage).unwrap().into())
    }
}
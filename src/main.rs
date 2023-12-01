use memmap2::{MmapMut, RemapOptions};
use serde::Serialize;
use std::{
    fs::{File, OpenOptions},
    mem::size_of,
    ops::Not,
};

fn main_2() {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open("./data")
        .unwrap();
    file.set_len(50).unwrap();

    let mut file = unsafe {
        memmap2::MmapOptions::new()
            // .len(1 * 1024 * 1024 * 1024)
            .map_mut(&file)
            // .map_anon()
            .unwrap()
    };

    // loop {}

    // let mut v = vec![];
    file[0..7].copy_from_slice(&[1, 1, 2, 2, 0, 0, 0]);

    println!("{:#?}", &file[..7]);

    println!("{:#?}", &file[..7]);

    //file[0..41].copy_from_slice(b"11111111111111111111111111111111111111111");

    //let data: &mut [u8] = &mut file;

    //data.split_at_mut(mid)

    println!("Done");
    loop {}
}

fn main() {
    let mut mystore = MapStore::create("./store", 10 * 1024 * 1024 * 1024);

    for i in make_deeta().take(10_000) {
        mystore.push(&i);
    }

    println!("Done");
    loop {}
}

pub fn make_deeta() -> impl Iterator<Item = String> {
    let mut i = 0;
    std::iter::from_fn(move || {
        let txt = format!("{i}_{i}_DATA").repeat(10 * i);
        i += 1;
        Some(txt)
    })
}

#[derive(Clone, Copy)]
pub struct MapStoreHeader {
    data_len: u32,
}

impl MapStoreHeader {
    pub fn new(data_len: u32) -> Self {
        MapStoreHeader { data_len }
    }

    pub fn len_bytes() -> usize {
        4
    }

    pub fn bytes(&self) -> [u8; 4] {
        self.data_len.to_le_bytes()
    }

    pub fn from_bytes(bytes: [u8; 4]) -> Self {
        Self::new(u32::from_le_bytes(bytes))
    }
}

pub struct MapStore {
    map: MmapMut,
    file: File,
    header: MapStoreHeader,
    capacity: usize,
}

impl MapStore {
    pub fn create(file: &str, initial_size: usize) -> Self {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(file)
            .unwrap();

        file.set_len(initial_size as u64).unwrap();

        let map = unsafe {
            memmap2::MmapOptions::new()
                .len(initial_size)
                .map_mut(&file)
                .unwrap()
        };

        let header = MapStoreHeader::new(0);

        Self {
            map,
            file,
            header,
            capacity: initial_size,
        }
    }

    fn write_header(&mut self, header: MapStoreHeader) {
        let len = MapStoreHeader::len_bytes();
        self.map[0..len].copy_from_slice(&header.bytes());
    }

    pub fn load(file: &str) -> Self {
        todo!()
        //
    }

    pub fn grow(&mut self, size: usize) {
        let new_len = self.capacity + size;

        self.file.set_len(new_len as u64).unwrap();
        unsafe {
            self.map.remap(new_len, RemapOptions::default()).unwrap();
        }

        self.capacity = new_len;
    }

    pub fn push<T: Serialize>(&mut self, item: &T) -> usize {
        let data = bincode::serialize(item).unwrap();
        self.push_raw(&data);
        data.len()
    }

    pub fn push_raw(&mut self, data: &[u8]) {
        self.next_slice(data.len()).copy_from_slice(&data);
        self.set_data_len(self.data_len() + data.len());
    }

    pub fn get(&self, index: usize, len: usize) -> Option<&[u8]> {
        let start = self.start_index() + index;
        let end = start + len;

        if end > self.start_index() + self.data_len() {
            return None;
        }

        Some(&self.map[start..end])
    }

    pub fn replace<T: Serialize>(&mut self, index: usize, len: usize, item: &T) -> usize {
        let data = bincode::serialize(item).unwrap();
        self.replace_raw(index, len, &data);
        data.len()
    }

    pub fn replace_raw(&mut self, index: usize, len: usize, data: &[u8]) {
        if data.len() == 0 {
            return;
        }

        if len == data.len() {
            let start = self.start_index() + index;
            let end = start + len;
            self.map[start..end].copy_from_slice(&data);
            return;
        }

        let shift_start_index = self.start_index() + index + len;

        if data.len() > len {
            let end_index = self.end_index();
            self.map
                .copy_within(shift_start_index..end_index, end_index);
            let start = self.start_index() + index;
            let end = start + data.len();
            self.map[start..end].copy_from_slice(data);
            self.set_data_len(self.data_len() + data.len() - len);
            return;
        }

        let end_index = self.end_index();
        let start_index = self.start_index();
        self.map.copy_within(
            shift_start_index..end_index,
            start_index + index + data.len(),
        );
        self.map[start_index + index..index + start_index + data.len()].copy_from_slice(data);
        self.set_data_len(self.data_len() - (len - data.len()));
    }

    pub fn data_len(&self) -> usize {
        self.header.data_len as usize
    }

    pub fn set_data_len(&mut self, len: usize) {
        self.header.data_len = len as u32;
        self.write_header(self.header);
    }

    pub fn start_index(&self) -> usize {
        MapStoreHeader::len_bytes()
    }

    pub fn end_index(&self) -> usize {
        self.start_index() + self.data_len()
    }

    pub fn free(&self) -> usize {
        self.capacity - self.data_len()
    }

    fn next_slice(&mut self, len: usize) -> &mut [u8] {
        let start = self.start_index() + self.data_len();
        let end = start + len;
        &mut self.map[start..end]
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use super::*;

    #[test]
    pub fn simple_push_get() {
        let mut map = MapStore::create("./test", 1024);

        let len = map.push(&1);
        assert_eq!(map.get(0, len).unwrap(), bincode::serialize(&1).unwrap());

        let len = map.push(&2);
        assert_eq!(map.get(len, len).unwrap(), bincode::serialize(&2).unwrap());
    }

    #[test]
    pub fn replace_same_len() {
        let mut map = MapStore::create("./test", 1024);

        let len = map.push(&1);
        assert_eq!(map.get(0, len).unwrap(), bincode::serialize(&1).unwrap());

        std::thread::sleep(Duration::from_millis(500));

        map.replace(0, len, &99);
        assert_eq!(map.get(0, len).unwrap(), bincode::serialize(&99).unwrap());
    }

    #[test]
    pub fn replace_bigger() {
        let mut map = MapStore::create("./test", 1024);

        let int_len = map.push(&1);
        assert_eq!(
            map.get(0, int_len).unwrap(),
            bincode::serialize(&1).unwrap()
        );

        let txt = "halloloolpenis";
        let str_len = map.replace(0, int_len, &txt);
        assert_eq!(
            map.get(0, str_len).unwrap(),
            bincode::serialize(&txt).unwrap()
        );
    }

    #[test]
    pub fn replace_smaler() {
        let mut map = MapStore::create("./test", 1024);

        let txt = "halloloolpenis";
        let str_len = map.push(&txt);
        assert_eq!(
            map.get(0, str_len).unwrap(),
            bincode::serialize(&txt).unwrap()
        );

        let int_len = map.replace(0, str_len, &1);
        assert_eq!(
            map.get(0, int_len).unwrap(),
            bincode::serialize(&1).unwrap()
        );

        assert_eq!(map.data_len(), 4);
    }
}

# Bytestore
Various data structures directly stored in memmapped files or as bytearray in memory with focus on efficiency and performance.

# Example usage
```rust
use bytestore::{
    backend::{
        memory::{MemoryBackend, MemoryData},
        mmap_mut::{MmapBackendMut, MmapFileMut},
    },
    components::{indexed_file::IndexedFile, map::FMap, multi_file::MultiFile},
    traits::creatable::Creatable,
};

fn main() {
    // Create a memory backend with an initial capacity of 100 bytes
    let backend = MemoryBackend::create(MemoryData::new(vec![0u8; 100])).unwrap();

    // Split the backend into multiple subackends to allow storing multiple variables of different
    // types in the same backend.
    let mut mfile = MultiFile::create(backend).unwrap();

    // Insert a new empty component of type `IndexedFile`. This data type holds an index for
    // multiple variable length data, like strings or lists. This can be seen as `Vec` with unknown
    // sized values.
    let mut my_list = mfile.insert_new_backend::<IndexedFile<_>>().unwrap();

    // Insert any serializable value!
    my_list.insert_t(&"Hello").unwrap();
    my_list.insert_t(&"world").unwrap();

    // And get them by their indices!
    assert_eq!(my_list.get_t::<String>(0).as_deref(), Ok("Hello"));
    assert_eq!(my_list.get_t::<String>(1).as_deref(), Ok("world"));

    // If the initial backend is a memory mapped file, you have to flush it in order to ensure its
    // completely written to disk. At this point you can drop it and load it later on.
    my_list.flush().unwrap();

    // Loading an existing file as backend mutable:
    let backend = MmapBackendMut::create(MmapFileMut::create("./my_file", 100).unwrap()).unwrap();
    // You can load it using:  let backend = MmapBackendMut::create(MmapFileMut::load("./my_file").unwrap()).unwrap();

    let mut mfile = MultiFile::create(backend).unwrap();

    // `FMap` is a HashMap like component that is stored in the given backend. It uses a custom
    // implementation of fnv. You have to implement the new trait for custom types.
    let mut hashmap = mfile.insert_new_backend::<FMap<_, String, u32>>().unwrap();
    hashmap.insert(&String::from("hello"), &1).unwrap();
    hashmap.insert(&String::from("world"), &2).unwrap();

    assert_eq!(hashmap.get(&String::from("hello")), Some(1));
    assert_eq!(hashmap.get(&String::from("world")), Some(2));
}
```

# Components
| Component name | Description |
| ----------- | ----------- |
| BitVec      | Bitvector that uses the provided backend to store the bits with as less memory as possible.  |
| CustomHeaderFile   | Implements "Backend" and can be used to store some metadata.         |
| IndexedFile | Similar to `Vec<T>` but additionally holds an index for variable sized data eg. strings. |
| List | Similar to `Vec<T>` but `T` is a fixed size type like integer. |
| CompressedIntList | List of integer but get serialized using varint. |
| FMap | Similar to HashMap. Uses fnv as hashing algorithm. |
| MultiFile | Splits a backend into multiple backends. Useful if you want to store multiple different components within the same backend. |
| SplitFile | Similar to MultiFile but only divides a backend into two backends. This has less overhead and you should prefer this one if you only need to split a backend into two. |

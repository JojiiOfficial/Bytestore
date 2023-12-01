use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mapstore::backend::memory::{MemoryBackend, MemoryData};
use mapstore::components::map::FMap;
use mapstore::traits::creatable::Creatable;
use std::time::{Duration, Instant};

fn benchme(c: &mut Criterion) {
    c.bench_function("map get none", |b| {
        let be = MemoryBackend::from_storage(MemoryData::new(vec![0u8; 100_000])).unwrap();
        let mut map: FMap<_, String, u32> = FMap::with_capacity(be, 100).unwrap();
        map.extend(make_deeta().take(500).map(|i| (i, 32)));

        let data = "holle".to_string();
        b.iter(|| {
            let _ = map.get(black_box(&data));
        });
    });

    c.bench_function("map get", |b| {
        let be = MemoryBackend::from_storage(MemoryData::new(vec![0u8; 100_000])).unwrap();
        let mut map: FMap<_, String, u32> = FMap::with_capacity(be, 100).unwrap();
        map.extend(make_deeta().take(500).map(|i| (i, 32)));

        let data = make_deeta().nth(3).unwrap();
        b.iter(|| {
            map.get(black_box(&data)).unwrap();
        });
    });

    c.bench_function("hashmap insert", |b| {
        let be = MemoryBackend::from_storage(MemoryData::new(vec![0u8; 100_000])).unwrap();
        let mut map: FMap<_, String, usize> = FMap::with_capacity(be, 100_000).unwrap();
        let data = black_box(make_deeta().skip(black_box(3)).take(1).next().unwrap());

        map.insert(&data, &11).unwrap();
        map.clear().unwrap();

        b.iter_custom(|i| {
            let mut dur = Duration::from_secs(0);

            for _ in 0..i {
                let start = Instant::now();
                map.insert(black_box(&data), &black_box(2314)).unwrap();
                let took = start.elapsed();
                dur += took;
                let _ = map.clear();
            }

            dur
        });
    });

    /*
    c.bench_function("put big", |b| {
        let mut map = MapStore::create("/cdisk/benchfile", 100 * 1024 * 1024).unwrap();
        let data = make_deeta().take(100).collect::<Vec<_>>().join(" ");
        b.iter_custom(|i| {
            let mut dur = Duration::default();
            for _ in 0..i {
                let start = Instant::now();
                let len = map.push(black_box(&data)).unwrap();
                dur += start.elapsed();
                map.pop(black_box(len));
            }

            dur
        });
    });
     */
}

pub fn make_deeta() -> impl Iterator<Item = String> {
    let mut i = 0;
    std::iter::from_fn(move || {
        let txt = format!("{i}_{i}_DATA").repeat(10 * i);
        i += 1;
        Some(txt)
    })
}

criterion_group!(benches, benchme);
criterion_main!(benches);

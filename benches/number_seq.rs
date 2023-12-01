use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mapstore::backend::memory::{MemoryBackend, MemoryData};
use mapstore::components::number_seq::NumberSequence;
use mapstore::traits::creatable::Creatable;
use std::collections::HashSet;

fn benchme(c: &mut Criterion) {
    c.bench_function("swap", |b| {
        let be = MemoryBackend::from_storage(MemoryData::new(vec![0u8; 8])).unwrap();
        let mut num_seq = NumberSequence::with_capacity(be, 0).unwrap();
        let rand_nrs = (0..1000).collect::<HashSet<_>>();
        num_seq.extend(rand_nrs.into_iter());

        b.iter(|| {
            let _ = num_seq.swap_unchecked(black_box(26), black_box(100));
        });
    });

    c.bench_function("sort", |b| {
        let be = MemoryBackend::from_storage(MemoryData::new(vec![0u8; 8])).unwrap();
        let mut num_seq = NumberSequence::with_capacity(be, 0).unwrap();
        let rand_nrs = (0..1000).collect::<HashSet<_>>();
        num_seq.extend(rand_nrs.into_iter());

        b.iter(|| {
            let _ = num_seq.sort();
        });
    });

    c.bench_function("get", |b| {
        let be = MemoryBackend::from_storage(MemoryData::new(vec![0u8; 8])).unwrap();
        let mut num_seq = NumberSequence::with_capacity(be, 0).unwrap();
        let rand_nrs = (0..1000).collect::<HashSet<_>>();
        num_seq.extend(rand_nrs.into_iter());

        b.iter(|| {
            let _ = num_seq.get(black_box(125));
        });
    });
}

criterion_group!(benches, benchme);
criterion_main!(benches);

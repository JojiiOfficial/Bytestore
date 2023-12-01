use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mapstore::components::bitvec::BitVec;
use mapstore::traits::creatable::{MemCreatable};

fn benchme(c: &mut Criterion) {
    c.bench_function("set at", |b| {
        let mut bv = BitVec::create_mem_with_capacity(1000).unwrap();
        bv.extend((0..10_000).map(|i| i % 43 == 0));

        b.iter(|| {
            let _ = bv.set(black_box(2361), black_box(true));
        });
    });

    c.bench_function("get", |b| {
        let mut bv = BitVec::create_mem_with_capacity(1000).unwrap();
        bv.extend((0..10_000).map(|i| i % 1000 == 0));

        b.iter(|| {
            let _ = bv.get(black_box(63));
        });
    });
}

criterion_group!(benches, benchme);
criterion_main!(benches);

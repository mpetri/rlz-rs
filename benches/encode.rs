use std::io::Read;

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use rlz::RlzCompressor;

pub fn encode_50_mb(c: &mut Criterion) {
    let mut e50 = std::fs::File::open("./data/english.50MB").unwrap();
    let mut e50_bytes = Vec::new();
    e50.read_to_end(&mut e50_bytes).unwrap();

    let mut dict_builder = rlz::Dictionary::reservoir_builder(4, 1024, 16);
    dict_builder.sample(&e50_bytes[..]);
    let dict = dict_builder.finish();

    let rlz_compressor = RlzCompressor::builder().build_from_dict(dict);

    let start = 1024 * 1024 * 16;
    let stop = start + (1024 * 1024);
    let encode_sample = &e50_bytes[start..stop];

    let mut output = Vec::with_capacity(1024 * 1024 * 256);
    let mut group = c.benchmark_group("english.50MB_4mb_dict");
    group.throughput(Throughput::Bytes(encode_sample.len() as u64));
    group.bench_function("encode", |b| {
        b.iter(|| rlz_compressor.encode(encode_sample, &mut output).unwrap())
    });
    group.finish();
}

criterion_group!(benches, encode_50_mb);
criterion_main!(benches);

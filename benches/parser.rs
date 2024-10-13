use criterion::{criterion_group, criterion_main, Criterion};
use std::{
    fs::File,
    hint::black_box,
    io::{BufReader, Read},
};
use templating_engine::{parse, parse_simd};

fn criterion_benchmark(c: &mut Criterion) {
    let mut f = BufReader::new(File::open("./test.txt").unwrap());
    let mut s = String::new();
    f.read_to_string(&mut s).unwrap();

    let mut g = c.benchmark_group("Parse Template");
    g.throughput(criterion::Throughput::Bytes(s.len() as u64));
    g.bench_function("newlines_simd", |b| {
        b.iter(|| parse_simd::parse_template(black_box(s.as_bytes())))
    });
    g.bench_function("newlines", |b| {
        b.iter(|| black_box(s.as_bytes()).iter().filter(|&&x| x == b'\n').count())
    });
    g.bench_function("parse", |b| {
        b.iter(|| parse::parse_template(black_box(s.as_bytes())))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

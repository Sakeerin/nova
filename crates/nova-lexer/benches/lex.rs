use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use nova_diagnostics::FileDb;
use nova_lexer::lex;

fn bench_lex(c: &mut Criterion) {
    let source = include_str!("../src/lib.rs"); // lex our own source as a proxy
    let mut db = FileDb::new();
    let file = db.add("<bench>", source);

    let mut group = c.benchmark_group("lexer");
    group.throughput(Throughput::Bytes(source.len() as u64));
    group.bench_function("lex_nova_source", |b| {
        b.iter(|| lex(black_box(source), black_box(file)));
    });
    group.finish();
}

criterion_group!(benches, bench_lex);
criterion_main!(benches);

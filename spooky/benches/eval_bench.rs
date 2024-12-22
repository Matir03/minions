use criterion::{black_box, criterion_group, criterion_main, Criterion};
use spooky::core::board::Board;
use spooky::captain::eval::evaluate;

fn eval_benchmark(c: &mut Criterion) {
    let board = Board::default();
    
    c.bench_function("position evaluation", |b| {
        b.iter(|| evaluate(black_box(&board)))
    });
}

criterion_group!(benches, eval_benchmark);
criterion_main!(benches);

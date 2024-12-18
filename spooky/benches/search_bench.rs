use criterion::{black_box, criterion_group, criterion_main, Criterion};
use spooky::core::{board::Board, action::Action};
use spooky::captain::search::search_position;

fn search_benchmark(c: &mut Criterion) {
    let board = Board {};  // TODO: Set up test position
    
    c.bench_function("search depth 3", |b| {
        b.iter(|| search_position(black_box(&board), black_box(3)))
    });
}

criterion_group!(benches, search_benchmark);
criterion_main!(benches);

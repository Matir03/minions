use criterion::{black_box, criterion_group, criterion_main, Criterion};
use spooky::ai::eval::Eval;
use spooky::core::{GameConfig, GameState};

fn eval_benchmark(c: &mut Criterion) {
    let config = GameConfig::default();
    let state = GameState::new_default(&config);

    c.bench_function("position evaluation", |b| {
        b.iter(|| Eval::static_eval(&config, black_box(&state)))
    });
}

criterion_group!(benches, eval_benchmark);
criterion_main!(benches);

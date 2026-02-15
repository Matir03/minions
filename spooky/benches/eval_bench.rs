use criterion::{criterion_group, criterion_main, Criterion};
use spooky::core::{GameConfig, GameState};
use spooky::heuristics::{naive::NaiveHeuristic, GeneralHeuristic, Heuristic};
use std::hint::black_box;

fn eval_benchmark(c: &mut Criterion) {
    let config = GameConfig::default();
    let state = GameState::new_default(&config);

    let heuristic = NaiveHeuristic::new(&config);
    let general_enc = <NaiveHeuristic as GeneralHeuristic<'_, _>>::compute_enc(
        &heuristic,
        state.side_to_move,
        &state.tech_state,
    );
    let shared = heuristic.compute_combined(&state, &general_enc, &[]);

    c.bench_function("position evaluation", |b| {
        b.iter(|| heuristic.compute_eval(black_box(&shared)))
    });
}

criterion_group!(benches, eval_benchmark);
criterion_main!(benches);

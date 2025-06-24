use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pprof::criterion::{PProfProfiler, Output};
use spooky::ai::search::SearchTree;
use spooky::core::{GameConfig, GameState};
use bumpalo::Bump;

fn search_position(config: &GameConfig, state: &GameState, explorations: u32) {
    let arena = Bump::new();
    let mut search_tree = SearchTree::new(config, state.clone(), &arena);
    for _ in 0..explorations {
        search_tree.explore();
    }
    // prevent the result from being optimized away
    black_box(search_tree.result());
}

fn search_benchmark(c: &mut Criterion) {
    let config = GameConfig::default();
    let state = GameState::default();
    let explorations = 100;

        c.bench_function(&format!("search_{}_explorations", explorations), |b| {
        b.iter(|| search_position(black_box(&config), black_box(&state), black_box(explorations)))
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = search_benchmark
}
criterion_main!(benches);

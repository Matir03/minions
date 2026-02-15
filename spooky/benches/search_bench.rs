use bumpalo::Bump;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pprof::criterion::{Output, PProfProfiler};
use spooky::ai::captain::board_node::Z3ContextStore;
use spooky::ai::explore::SearchTree;
use spooky::core::{GameConfig, GameState};
use spooky::heuristics::{naive::NaiveHeuristic, Heuristic};

fn search_position<'a, H: Heuristic<'a>>(
    config: &'a GameConfig,
    state: &'a GameState,
    arena: &'a Bump,
    heuristic: &'a H,
    ctx_store: &'a Z3ContextStore,
    explorations: u32,
) {
    let mut search_tree = SearchTree::new(config, state.clone(), arena, heuristic);
    for _ in 0..explorations {
        search_tree.explore(config, arena, heuristic, ctx_store);
    }
    // prevent the result from being optimized away
    black_box(search_tree.result());
}

fn search_benchmark(c: &mut Criterion) {
    let config = GameConfig::default();
    let state = GameState::new_default(&config);
    let arena = Bump::new();
    let heuristic = NaiveHeuristic::new(&config);
    let ctx_store = Z3ContextStore::new(Vec::new());
    let explorations = 100;

    c.bench_function(&format!("search_{}_explorations", explorations), |b| {
        b.iter(|| {
            search_position(
                black_box(&config),
                black_box(&state),
                black_box(&arena),
                black_box(&heuristic),
                black_box(&ctx_store),
                black_box(explorations),
            )
        })
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)));
    targets = search_benchmark
}
criterion_main!(benches);

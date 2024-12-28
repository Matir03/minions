use rand::{prelude::*, rngs::StdRng};

#[cfg(debug_assertions)]
pub fn make_rng() -> StdRng {
    const SEED: u64 = 63;
    StdRng::seed_from_u64(SEED)   
}

#[cfg(not(debug_assertions))]
pub fn make_rng() -> StdRng {
    StdRng::from_entropy()
}
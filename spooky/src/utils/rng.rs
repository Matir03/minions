use rand::{prelude::*, rngs::StdRng, rngs::SysRng};

#[cfg(debug_assertions)]
pub fn make_rng() -> StdRng {
    const SEED: u64 = 63;
    StdRng::seed_from_u64(SEED)
}

#[cfg(not(debug_assertions))]
pub fn make_rng() -> StdRng {
    use rand::TryRng;
    let seed = SysRng::try_next_u64(&mut SysRng).unwrap();

    StdRng::seed_from_u64(seed)
}

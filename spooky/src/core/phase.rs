#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Attack,
    Spawn,
}

impl Phase {
    pub fn is_attack(&self) -> bool {
        *self == Phase::Attack
    }

    pub fn is_spawn(&self) -> bool {
        *self == Phase::Spawn
    }
}

//! Bitboards
use super::loc::Loc;

pub type Bitboard = u128;


#[derive(Copy, Clone)]
struct Dir {
    dx: i32,
    dy: i32,
}

impl Dir {
    const fn shift(&self) -> i32 {
        self.dy * 10 + self.dx
    }

    const fn make_mask(&self) -> Bitboard {
        let mut mask: Bitboard = 0;
        let mut idx = -1;

        loop {
            idx += 1;

            if idx >= 100 {
                break;
            }

            let loc: Loc = Loc { 
                y: idx / 10 + self.dx, 
                x: idx % 10 + self.dy 
            };

            if !loc.in_bounds() { continue; }

            let shift = self.shift();
            let new_idx = idx + shift;

            if 0 <= new_idx && new_idx < 128 {
                mask |= 1 << new_idx;
            }   
        }

        mask
    }
}

const DIRECTIONS: [Dir; 6] = [
    Dir { dx: -1, dy: 0 }, // WEST
    Dir { dx: -1, dy: 1 }, // NW
    Dir { dx: 0, dy: -1 }, // SW
    Dir { dx: 0, dy: 1 }, // NE
    Dir { dx: 1, dy: -1 }, // SE
    Dir { dx: 1, dy: 0 }, // EAST
];

// const SHIFTS: [i32; 6] = [
//     DIRECTIONS[0].shift(),
//     DIRECTIONS[1].shift(),
//     DIRECTIONS[2].shift(),
//     DIRECTIONS[3].shift(),
//     DIRECTIONS[4].shift(),
//     DIRECTIONS[5].shift(),
// ];

const MASKS: [Bitboard; 6] = [
    DIRECTIONS[0].make_mask(),
    DIRECTIONS[1].make_mask(),
    DIRECTIONS[2].make_mask(),
    DIRECTIONS[3].make_mask(),
    DIRECTIONS[4].make_mask(),
    DIRECTIONS[5].make_mask(),
];

pub trait BitboardOps {
    fn new() -> Self;

    fn propagate(&self) -> Self;
    fn propagate_masked(&self, mask: Bitboard) -> Self;
    fn all_movements(&self, range: i32, prop: Bitboard, vacant: Bitboard) -> Self;

    fn get(&self, loc: Loc) -> bool;
    fn set(&mut self, loc: Loc, value: bool);
}

impl BitboardOps for Bitboard {

    fn new() -> Self { 0 }

    fn propagate(&self) -> Self {
        ((self >>  1) & MASKS[0]) |
        ((self <<  9) & MASKS[1]) |
        ((self >> 10) & MASKS[2]) |
        ((self << 10) & MASKS[3]) |
        ((self >>  9) & MASKS[4]) |
        ((self <<  1) & MASKS[5])
    }

    fn propagate_masked(&self, mask: Bitboard) -> Self {
        self.propagate() & mask
    }

    fn all_movements(&self, range: i32, prop: Bitboard, vacant: Bitboard) -> Self {
        let mut moves: Bitboard = *self;

        for _ in 0..range {
            moves |= moves.propagate_masked(prop);
        }

        moves & vacant
    }

    fn get(&self, loc: Loc) -> bool {
        self & (1 << loc.index()) != 0
    }

    fn set(&mut self, loc: Loc, value: bool) {
        if value {
            *self |= 1 << loc.index();
        } else {
            *self &= !(1 << loc.index());
        }
    }
}
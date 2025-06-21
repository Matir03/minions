//! Bitboards
use super::{loc::Loc, map::{MapSpec, TileType, Terrain}, side::Side};

// Helper function to convert a bitboard to a Vec<Loc>
pub fn bitboard_to_locs(bb: Bitboard) -> Vec<Loc> {
    let mut locs = Vec::new();
    for i in 0..100 {
        if (bb >> i) & 1 != 0 {
            locs.push(Loc::from_index(i));
        }
    }
    locs
}

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
        let mut idx = 0;

        while idx < 100 {
            let loc: Loc = Loc { 
                y: (idx / 10) + self.dy, 
                x: (idx % 10) + self.dx 
            };

            if loc.in_bounds() {
                let shift = self.shift();
                let new_idx = idx + shift;
    
                if new_idx >= 0 && new_idx < 100 {
                    mask |= 1 << new_idx;
                }
            }
            idx += 1;
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

#[derive(Debug, Clone)]
pub struct Bitboards {
    pub s0_pieces: Bitboard,
    pub s1_pieces: Bitboard,
    pub all_pieces: Bitboard,

    // Terrain bitboards
    pub water: Bitboard,
    pub earthquake: Bitboard,
    pub whirlwind: Bitboard,
    pub firestorm: Bitboard,
}

impl Bitboards {
    pub fn new(map_spec: &MapSpec) -> Self {
        let mut water = Bitboard::new();
        let mut earthquake = Bitboard::new();
        let mut whirlwind = Bitboard::new();
        let mut firestorm = Bitboard::new();

        for y in 0..10 {
            for x in 0..10 {
                let loc = Loc::new(x, y);
                if let Some(tile) = map_spec.tiles.get(&loc) {
                    if let TileType::NativeTerrain(terrain) = tile {
                        match terrain {
                            Terrain::Flood => water.set(loc, true),
                            Terrain::Earthquake => earthquake.set(loc, true),
                            Terrain::Whirlwind => whirlwind.set(loc, true),
                            Terrain::Firestorm => firestorm.set(loc, true),
                        }
                    }
                }
            }
        }

        Self {
            s0_pieces: Bitboard::new(),
            s1_pieces: Bitboard::new(),
            all_pieces: Bitboard::new(),
            water,
            earthquake,
            whirlwind,
            firestorm,
        }
    }

    pub fn set_piece(&mut self, loc: &Loc, side: Side, value: bool) {
        match side {
            Side::S0 => self.s0_pieces.set(*loc, value),
            Side::S1 => self.s1_pieces.set(*loc, value),
        };
        self.all_pieces.set(*loc, value);
    }

    pub fn get_valid_moves(&self, loc: &Loc, side: Side, range: i32, is_flying: bool) -> Bitboard {
        let mut prop_mask = !self.all_pieces;
        prop_mask.set(*loc, true);

        let dest_mask = if side == Side::S0 {
            !self.s0_pieces
        } else {
            !self.s1_pieces
        };

        if !is_flying {
            prop_mask &= !self.water;
        }

        let mut start = Bitboard::new();
        start.set(*loc, true);

        start.all_movements(range, prop_mask, dest_mask)
    }

    pub fn get_valid_move_hexes(&self, loc: &Loc, side: Side, range: i32, is_flying: bool) -> Vec<Loc> {
        let moves_bb = self.get_valid_moves(loc, side, range, is_flying);
        bitboard_to_locs(moves_bb)
    }
}
    
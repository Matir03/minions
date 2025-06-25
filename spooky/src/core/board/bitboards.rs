//! Bitboards
use crate::core::{
    board::Piece, loc::Loc, map::{MapSpec, Terrain, TileType}, side::{Side, SideArray}, Board
};

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
            let cur_loc = Loc { 
                y: (idx / 10), 
                x: (idx % 10),
            };
            let new_loc = Loc {
                y: cur_loc.y + self.dy,
                x: cur_loc.x + self.dx,
            };

            if new_loc.in_bounds() {
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
    fn to_locs(self) -> Vec<Loc>;

    fn neighbors(&self) -> Self;
    fn propagate_masked(&self, mask: Bitboard) -> Self;
    fn all_movements(&self, speed: i32, prop: Bitboard, vacant: Bitboard) -> Self;

    fn get(&self, loc: Loc) -> bool;
    fn set(&mut self, loc: Loc, value: bool);
    fn pop(&mut self) -> Option<Loc>;
}

impl BitboardOps for Bitboard {

    fn new() -> Self { 0 }

    fn to_locs(self) -> Vec<Loc> {
        let mut locs = Vec::new();
        let mut bb = self;
        while bb != 0 {
            let bit_index = bb.trailing_zeros() as i32;
            locs.push(Loc::from_index(bit_index));
            bb &= bb - 1; // Clear the lowest set bit
        }
        locs
    }

    fn neighbors(&self) -> Self {
        ((self >>  1) & MASKS[0]) |
        ((self <<  9) & MASKS[1]) |
        ((self >> 10) & MASKS[2]) |
        ((self << 10) & MASKS[3]) |
        ((self >>  9) & MASKS[4]) |
        ((self <<  1) & MASKS[5])
    }

    fn propagate_masked(&self, mask: Bitboard) -> Self {
        self.neighbors() & mask
    }

    fn all_movements(&self, speed: i32, prop: Bitboard, vacant: Bitboard) -> Self {
        let mut moves: Bitboard = *self;

        for _ in 0..speed {
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

    fn pop(&mut self) -> Option<Loc> {
        if *self == 0 { return None; }
        let loc = self.trailing_zeros() as i32;
        *self &= *self - 1; // Clear the lowest set bit
        Some(Loc::from_index(loc))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bitboards {
    // pieces by type
    pub pieces: SideArray<Bitboard>,
    pub spawners: SideArray<Bitboard>,

    // Terrain bitboards
    pub water: Bitboard,
    pub land: Bitboard,

    pub earthquake: Bitboard,
    pub whirlwind: Bitboard,
    pub firestorm: Bitboard,
    pub graveyards: Bitboard,
}

impl Bitboards {
    pub fn new(map_spec: &MapSpec) -> Self {
        let mut water = Bitboard::new();
        let mut land = Bitboard::new();
        let mut earthquake = Bitboard::new();
        let mut whirlwind = Bitboard::new();
        let mut firestorm = Bitboard::new();
        let mut graveyards = Bitboard::new();

        for y in 0..10 {
            for x in 0..10 {
                let loc = Loc::new(x, y);
                if let Some(tile) = map_spec.tiles.get(&loc) {
                    match tile {
                        TileType::NativeTerrain(terrain) => match terrain {
                            Terrain::Flood => water.set(loc, true),
                            Terrain::Earthquake => earthquake.set(loc, true),
                            Terrain::Whirlwind => whirlwind.set(loc, true),
                            Terrain::Firestorm => firestorm.set(loc, true),
                        },
                        TileType::Graveyard => graveyards.set(loc, true),
                        _ => {},
                    }
                }
            }
        }

        Self {
            pieces: SideArray::new(Bitboard::new(), Bitboard::new()),
            spawners: SideArray::new(Bitboard::new(), Bitboard::new()),
            water,
            land: !water,
            earthquake,
            whirlwind,
            firestorm,
            graveyards,
        }
    }

    pub fn add_piece(&mut self, piece: &Piece) {
        let loc = piece.loc;
        let side = piece.side;
    
        self.pieces[side].set(loc, true);

        if piece.unit.stats().spawn {
            self.spawners[side].set(loc, true);
        }
    }

    pub fn remove_piece(&mut self, loc: &Loc, side: Side) {
        self.pieces[side].set(*loc, false);
        self.spawners[side].set(*loc, false);
    }

    pub fn empty(&self) -> Bitboard {
        !(self.pieces[Side::S0] | self.pieces[Side::S1])
    }

    pub fn get_valid_moves(&self, loc: &Loc, side: Side, speed: i32, is_flying: bool) -> Bitboard {
        let mut prop_mask = !self.pieces[!side];
        if !is_flying {
            prop_mask &= !self.water;
        }

        let mut dest_mask = self.empty();
        dest_mask.set(*loc, true);

        let mut start = Bitboard::new();
        start.set(*loc, true);

        start.all_movements(speed, prop_mask, dest_mask)
    }

    pub fn get_unobstructed_moves(&self, loc: &Loc, side: Side, speed: i32, is_flying: bool) -> Bitboard {
        let mut prop_mask = !self.pieces[!side];
        if !is_flying {
            prop_mask &= !self.water;
        }

        let mut dest_mask = !0;

        let mut start = Bitboard::new();
        start.set(*loc, true);

        start.all_movements(speed, prop_mask, dest_mask)
    }

    pub fn get_theoretical_moves(&self, loc: &Loc, side: Side, speed: i32, is_flying: bool) -> Bitboard {
        let mut prop_mask = if is_flying { !0 } else { !self.water };
        let dest_mask = !0;

        let mut start = Bitboard::new();
        start.set(*loc, true);

        start.all_movements(speed, prop_mask, dest_mask)
    }

    pub fn occupied_graveyards(&self, side: Side) -> Bitboard {
        self.graveyards & self.pieces[side]
    }

    pub fn unoccupied_graveyards(&self, side: Side) -> Bitboard {
        self.graveyards & !self.pieces[side]
    }

    pub fn get_spawn_locs(&self, side: Side, flying: bool) -> Bitboard {
        let mut hexes = self.spawners[side].neighbors();
        hexes &= self.empty();
        if !flying {
            hexes &= self.land;
        }
        hexes
    }
}

impl<'a> Board<'a> {
    pub fn units_on_graveyards(&self, side: Side) -> i32 {
        self.bitboards.occupied_graveyards(side).count_ones() as i32
    }

    pub fn unoccupied_graveyards(&self, side: Side) -> Vec<Loc> {
        self.bitboards.unoccupied_graveyards(side).to_locs()
    }

    pub fn get_unobstructed_moves(&self, piece_loc: Loc) -> Bitboard {
        let piece = self.get_piece(&piece_loc).unwrap();
        let speed = piece.unit.stats().speed;
        let is_flying = piece.unit.stats().flying;
        self.bitboards.get_unobstructed_moves(&piece_loc, piece.side, speed, is_flying)
    }

    pub fn get_theoretical_moves(&self, piece_loc: Loc) -> Bitboard {
        let piece = self.get_piece(&piece_loc).unwrap();
        let speed = piece.unit.stats().speed;
        let is_flying = piece.unit.stats().flying;
        self.bitboards.get_theoretical_moves(&piece_loc, piece.side, speed, is_flying)
    }

    pub fn get_valid_moves(&self, piece_loc: Loc) -> Bitboard {
        let piece = self.get_piece(&piece_loc).unwrap();
        let speed = piece.unit.stats().speed;
        let is_flying = piece.unit.stats().flying;
        self.bitboards.get_valid_moves(&piece_loc, piece.side, speed, is_flying)
    }

    pub fn get_valid_move_hexes(&self, piece_loc: Loc) -> Vec<Loc> {
        self.get_valid_moves(piece_loc).to_locs()
    }

    pub fn get_theoretical_move_hexes(&self, piece_loc: Loc) -> Vec<Loc> {
        self.get_theoretical_moves(piece_loc).to_locs()
    }

    pub fn get_unobstructed_move_hexes(&self, piece_loc: Loc) -> Vec<Loc> {
        self.get_unobstructed_moves(piece_loc).to_locs()
    }

    /// Returns a list of valid, empty locations where the given side can spawn units.
    pub fn get_spawn_locs(&self, side: Side, flying: bool) -> Vec<Loc> {
        self.bitboards.get_spawn_locs(side, flying).to_locs()
    }
}
    
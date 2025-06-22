//! Board representation and rules

use std::{cell::RefCell, collections::HashMap};
use anyhow::{anyhow, bail, ensure, Context, Result};
use hashbag::HashBag;
use crate::core::loc::GRID_LEN;

use super::{
    action::BoardAction,
    bitboards::{Bitboard, BitboardOps, Bitboards},
    loc::{Loc, PATH_MAPS},
    map::{Map, MapSpec, Terrain},
    phase::Phase,
    side::{Side, SideArray},
    spells::Spell,
    tech::TechState,
    units::{Unit, Attack},
};

/// Status modifiers that can be applied to pieces
#[derive(Debug, Clone, Default)]
pub struct Modifiers {
    // pub shielded: bool,
    // pub frozen: bool,
    // pub shackled: bool,
    // TODO: Add other modifiers
}

#[derive(Debug, Clone, Default, Copy)]
pub struct PieceState {
    pub moved: bool,
    pub attacks_used: i32,
    pub damage_taken: i32,
    pub exhausted: bool,
}

impl PieceState {
    pub fn spawned() -> Self {
        Self {
            moved: false,
            attacks_used: 0,
            damage_taken: 0,
            exhausted: true,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardState {
    Normal,
    Reset0,
    Reset1,
    Reset2,
}

impl Default for BoardState {
    fn default() -> Self {
        Self::Normal
    }
}

/// Represents a piece on the board
#[derive(Debug, Clone)]
pub struct Piece {
    pub loc: Loc,
    pub side: Side,
    pub unit: Unit,
    pub modifiers: Modifiers,
    pub state: RefCell<PieceState>,
}

impl Piece {
    pub fn new(unit: Unit, side: Side, loc: Loc) -> Self {
        Self {
            unit,
            side,
            loc,
            modifiers: Modifiers::default(),
            state: RefCell::new(PieceState::default()),
        }
    }
}

/// Represents a single Minions board
#[derive(Debug, Clone)]
pub struct Board {
    pub map: Map,
    pub pieces: HashMap<Loc, Piece>,
    pub reinforcements: SideArray<HashBag<Unit>>,
    pub spells: SideArray<HashBag<Spell>>,
    pub winner: Option<Side>,
    pub state: BoardState,
    pub bitboards: Bitboards,
}

const GRAVEYARDS_TO_WIN: i32 = 8;

impl Board {
    pub const START_FEN: &str = "10/2ZZ6/1ZNZ6/1ZZ7/10/10/7zz1/6znz1/6zz2/10";

    /// Create a new empty board
    pub fn new(map: Map) -> Self {
        Self {
            map,
            pieces: HashMap::new(),
            reinforcements: SideArray::new(HashBag::new(), HashBag::new()),
            spells: SideArray::new(HashBag::new(), HashBag::new()),
            winner: None,
            state: BoardState::default(),
            bitboards: Bitboards::new(map.spec()),
        }
    }

    pub fn get_piece(&self, loc: &Loc) -> Option<&Piece> {
        self.pieces.get(loc)
    }

    /// Add a piece to the board
    pub fn add_piece(&mut self, piece: Piece) {
        debug_assert!(!self.pieces.contains_key(&piece.loc));
        self.bitboards.add_piece(&piece);
        self.pieces.insert(piece.loc, piece);
    }

    /// Remove a piece from the board
    pub fn remove_piece(&mut self, loc: &Loc) -> Option<Piece> {
        if let Some(piece) = self.pieces.remove(loc) {
            self.bitboards.remove_piece(loc, piece.side);
            Some(piece)
        } else {
            None
        }
    }

    pub fn bounce_piece(&mut self, piece: Piece) {
        self.add_reinforcement(piece.unit, piece.side);
    }

    pub fn add_reinforcement(&mut self, unit: Unit, side: Side) {
        self.reinforcements[side].insert(unit);
    }

    pub fn can_move(&self, from_loc: &Loc, to_loc: &Loc, side_to_move: Side) -> Result<()> {
        let piece = self
            .get_piece(from_loc)
            .ok_or_else(|| anyhow!("No piece to move from {}", from_loc))?;

        ensure!(piece.side == side_to_move, "Cannot move opponent's piece");
        ensure!(!piece.state.borrow().moved, "Cannot move piece twice");

        let valid_moves = self.get_valid_move_hexes(*from_loc);
        ensure!(
            valid_moves.contains(to_loc),
            "No valid path from {} to {}",
            from_loc,
            to_loc
        );

        Ok(())
    }

    fn move_piece(&mut self, mut piece: Piece, to_loc: &Loc) {
        piece.loc = *to_loc;
        piece.state.borrow_mut().moved = true;

        self.add_piece(piece);
    }

    fn try_attack(&self, attacker_loc: Loc, target_loc: Loc, side_to_move: Side) -> Result<(bool, bool)> {
        let attacker = self
            .get_piece(&attacker_loc)
            .ok_or_else(|| anyhow!("No attacker at {attacker_loc}"))?;

        ensure!(attacker.side == side_to_move, "Cannot attack with opponent's piece");

        let attacker_stats = attacker.unit.stats();
        let mut attacker_state = attacker.state.borrow_mut();

        ensure!(
            !attacker_state.moved || !attacker_stats.lumbering,
            "Lumbering piece cannot move and attack on the same turn"
        );

        ensure!(
            attacker_loc.dist(&target_loc) <= attacker_stats.range,
            "Attack range not large enough"
        );

        ensure!(
            attacker_state.attacks_used < attacker_stats.num_attacks,
            "Piece has already used all attacks"
        );

        let target = self
            .get_piece(&target_loc)
            .ok_or_else(|| anyhow!("No target at {target_loc}"))?;

        let target_stats = target.unit.stats();

        ensure!(target.side != side_to_move, "Cannot attack own piece");

        let attack = attacker_stats.attack;
        let mut damage_effect: i32 = 0;
        let mut bounce = false;

        match attack {
            Attack::Damage(damage) => {
                damage_effect = damage;
            }
            Attack::Deathtouch => {
                ensure!(!target_stats.necromancer, "Deathtouch cannot be used on necromancer");
                damage_effect = target_stats.defense;
            }
            Attack::Unsummon => {
                if target_stats.persistent {
                    damage_effect = 1
                } else {
                    bounce = true;
                }
            }
        }

        attacker_state.moved = true;
        attacker_state.attacks_used += 1;

        let mut target_state = target.state.borrow_mut();

        target_state.damage_taken += damage_effect;

        let remove = target_state.damage_taken >= target_stats.defense || bounce;

        Ok((remove, bounce))
    }

    pub fn units_on_graveyards(&self, side: Side) -> i32 {
        self.bitboards.occupied_graveyards(side).count_ones() as i32
    }

    pub fn unoccupied_graveyards(&self, side: Side) -> Vec<Loc> {
        self.bitboards.unoccupied_graveyards(side).to_locs()
    }

    pub fn do_action(
        &mut self,
        action: BoardAction,
        money: &mut SideArray<i32>,
        board_points: &mut SideArray<i32>,
        tech_state: &TechState,
        side_to_move: Side,
        phase: Phase,
    ) -> Result<()> {
        if let Some(winner) = self.winner {
            match action {
                BoardAction::SaveUnit { unit } => {
                    if let Some(unit) = unit {
                        ensure!(!unit.stats().necromancer, "Cannot save necromancer");

                        ensure!(
                            self.reinforcements[side_to_move].contains(&unit) > 0,
                            "Cannot save unit not in reinforcements"
                        );

                        self.reinforcements[side_to_move].clear();
                        self.reinforcements[side_to_move].insert(unit);
                    } else {
                        self.reinforcements[side_to_move].clear();
                    }

                    if winner != side_to_move {
                        self.winner = None;
                    }
                }

                _ => bail!("Only SaveUnit action allowed on board yet to be restarted"),
            }

            return Ok(());
        }

        match self.state {
            BoardState::Normal => self.do_normal_action(action, money, board_points, tech_state, side_to_move, phase),
            BoardState::Reset0 => bail!("No actions allowed in Reset0"),
            BoardState::Reset1 | BoardState::Reset2 => {
                let side = if self.state == BoardState::Reset1 { Side::S0 } else { Side::S1 };
                ensure!(side == side_to_move, "Wrong side to move for reset");
                match action {
                    BoardAction::Spawn { spawn_loc, unit } => {
                        ensure!(unit.stats().necromancer, "Can only spawn necromancer in reset");
                        ensure!(self.reinforcements[side].contains(&unit) > 0, "Necromancer not in reinforcements");
                        ensure!(self.map.spec().graveyards.contains(&spawn_loc), "Can only spawn necromancer on graveyard");
                        ensure!(self.get_piece(&spawn_loc).is_none(), "Can't spawn on occupied square");

                        self.reinforcements[side].remove(&unit);
                        self.add_piece(Piece::new(unit, side, spawn_loc));
                        Ok(())
                    }
                    _ => bail!("Only spawn necromancer action is allowed in reset"),
                }
            }
        }
    }

    fn do_normal_action(
        &mut self,
        action: BoardAction,
        money: &mut SideArray<i32>,
        board_points: &mut SideArray<i32>,
        tech_state: &TechState,
        side_to_move: Side,
        phase: Phase,
    ) -> Result<()> {
        match action {
            BoardAction::Move { from_loc, to_loc } => {
                ensure!(phase.is_attack(), "Cannot move in spawn phase");
                ensure!(self.get_piece(&to_loc).is_none(), "Cannot move to occupied square");

                self.can_move(&from_loc, &to_loc, side_to_move)?;

                let piece = self.remove_piece(&from_loc).unwrap();
                self.move_piece(piece, &to_loc);
            }
            BoardAction::MoveCyclic { locs } => {
                ensure!(phase.is_attack(), "Cannot move in spawn phase");

                let num_locs = locs.len();
                ensure!(num_locs >= 2, "MoveCyclic must have at least 2 locations");

                // Verify all moves are valid first
                for i in 0..num_locs {
                    let from_loc = &locs[i];
                    let to_loc = &locs[(i + 1) % num_locs];
                    self.can_move(from_loc, to_loc, side_to_move)?;
                }

                // Store first piece
                let first_loc = &locs[0];
                let mut current_piece = self.remove_piece(first_loc).unwrap();

                // Perform the cyclic move
                for i in 1..num_locs {
                    let next_loc = &locs[i];

                    // For all but the last move, we need to swap pieces
                    let next_piece = self.remove_piece(next_loc).unwrap();

                    self.move_piece(current_piece, next_loc);
                    current_piece = next_piece;
                }

                // Add the last piece
                self.move_piece(current_piece, first_loc);
            }
            BoardAction::Attack { attacker_loc, target_loc } => {
                ensure!(phase.is_attack(), "Cannot attack in spawn phase");

                let (remove, bounce) =
                    self.try_attack(attacker_loc, target_loc, side_to_move)?;

                if !remove {
                    return Ok(());
                }

                let target_piece = self.remove_piece(&target_loc).unwrap();

                if target_piece.unit.stats().necromancer {
                    self.win(side_to_move, board_points);
                } else if bounce {
                    self.bounce_piece(target_piece);
                } else {
                    money[!side_to_move] += target_piece.unit.stats().rebate;
                }
            }
            BoardAction::Blink { blink_loc: blink_sq } => {
                if let Some(piece) = self.get_piece(&blink_sq) {
                    ensure!(piece.side == side_to_move, "Cannot blink enemy piece");
                    ensure!(piece.unit.stats().blink, "Piece does not have blink ability");
                } else {
                    bail!("No piece at blink square")
                }

                let piece = self.remove_piece(&blink_sq).unwrap();
                self.bounce_piece(piece);
            }
            BoardAction::Buy { unit } => {
                ensure!(phase.is_spawn(), "Cannot buy in attack phase");

                ensure!(tech_state.can_buy(side_to_move, unit), "Unit not teched to");

                let cost = unit.stats().cost;
                ensure!(*money.get(side_to_move)? >= cost, "Not enough money");

                money[side_to_move] -= cost;
                self.add_reinforcement(unit, side_to_move);
            }
            BoardAction::Spawn { spawn_loc, unit } => {
                ensure!(phase.is_spawn(), "Cannot spawn in attack phase");

                ensure!(self.get_piece(&spawn_loc).is_none(), "Can't spawn on occupied square");

                ensure!(
                    self.reinforcements[side_to_move].contains(&unit) > 0,
                    "Cannot spawn unit not in reinforcements"
                );

                // Check for a spawner
                let mut has_spawner = false;
                for neighbor in spawn_loc.neighbors() {
                    if let Some(piece) = self.get_piece(&neighbor) {
                        if piece.side == side_to_move
                            && piece.unit.stats().spawn
                            && !piece.state.borrow().exhausted
                        {
                            has_spawner = true;
                            break;
                        }
                    }
                }
                ensure!(has_spawner, "No spawner in range");

                self.reinforcements[side_to_move].remove(&unit);
                let mut piece = Piece::new(unit, side_to_move, spawn_loc);
                piece.state.borrow_mut().exhausted = true;
                self.add_piece(piece);
            }
            BoardAction::SaveUnit { .. } => bail!("SaveUnit not allowed in normal play"),
            BoardAction::Cast { .. } => bail!("Cast not allowed in normal play"),
            BoardAction::Discard { .. } => bail!("Discard not allowed in normal play"),
            BoardAction::EndPhase => bail!("EndPhase not allowed in normal play"),
            BoardAction::Resign => bail!("Resign not allowed in normal play"),
        }
        Ok(())
    }

    pub fn end_turn(
        &mut self,
        money: &mut SideArray<i32>,
        board_points: &mut SideArray<i32>,
        side_to_move: Side,
    ) -> Result<()> {
        match self.state {
            BoardState::Normal => {
                // Reset piece states
                for piece in self.pieces.values() {
                    piece.state.borrow_mut().reset();
                }

                // Income
                let income = self.units_on_graveyards(side_to_move) + 2;
                money[side_to_move] += income;

                // Check for graveyard control loss
                if self.units_on_graveyards(!side_to_move) >= GRAVEYARDS_TO_WIN {
                    self.lose(side_to_move, board_points);
                }
            }
            BoardState::Reset0 => {
                self.state = BoardState::Reset1;
            }
            BoardState::Reset1 => {
                self.state = BoardState::Reset2;
            }
            BoardState::Reset2 => {
                // Spawn zombies around necromancers for both sides
                for side in Side::all() {
                    if let Some(necro_loc) = self.find_necromancer(side) {
                        for neighbor in necro_loc.neighbors() {
                            if self.get_piece(&neighbor).is_none() {
                                self.add_piece(Piece::new(Unit::Zombie, side, neighbor));
                            }
                        }
                    }
                }
                self.state = BoardState::Normal;
            }
        }

        Ok(())
    }

    pub fn assign_spell(&mut self, spell: Spell, side: Side) -> Result<()> {
        self.spells[side].insert(spell);
        Ok(())
    }

    pub fn win(&mut self, side: Side, board_points: &mut SideArray<i32>) {
        if self.winner.is_none() {
            self.winner = Some(side);
            board_points[side] += 1;
        }
    }

    pub fn lose(&mut self, side: Side, board_points: &mut SideArray<i32>) {
        if self.winner.is_none() {
            self.winner = Some(!side);
            board_points[!side] += 1;
        }
    }

    pub fn clear(&mut self) {
        self.pieces.clear();
        self.bitboards = Bitboards::new(self.map.spec());
    }

    pub fn reset(&mut self) {
        let to_bounce = 
            self.pieces.drain()
                .filter(|(_, piece)| !piece.unit.stats().necromancer)
                .collect::<Vec<_>>();

        *self = Board::from_fen(Self::START_FEN, self.map).unwrap();

        for (_, piece) in to_bounce {
            self.bounce_piece(piece);
        }
    }

    fn find_necromancer(&self, side: Side) -> Option<Loc> {
        self.pieces.values()
            .find(|p| p.side == side && p.unit.stats().necromancer)
            .map(|p| p.loc)
    }

    // / Convert board state to FEN notation
    pub fn to_fen(&self) -> String {
        let mut fen = String::new();
        for y in 0..GRID_LEN as i32 {
            let mut empty_count = 0;
            for x in 0..GRID_LEN as i32 {
                let loc = Loc::new(x, y);
                if let Some(piece) = self.get_piece(&loc) {
                    if empty_count > 0 {
                        fen.push_str(&empty_count.to_string());
                        empty_count = 0;
                    }
                    let mut c = piece.unit.to_fen_char();
                    if piece.side == Side::S1 {
                        c = c.to_ascii_lowercase();
                    }
                    fen.push(c);
                } else {
                    empty_count += 1;
                }
            }
            if empty_count > 0 {
                fen.push_str(&empty_count.to_string());
            }
            if y < GRID_LEN as i32 - 1 {
                fen.push('/');
            }
        }
        fen
    }

    // / Create a board from FEN notation
    pub fn from_fen(fen: &str, map: Map) -> Result<Self> {
        let mut board = Self::new(map);
        let rows: Vec<&str> = fen.split('/').collect();
        ensure!(rows.len() == GRID_LEN, "FEN must have {} rows, but has {}", GRID_LEN, rows.len());

        for (y, row_str) in rows.iter().enumerate() {
            let mut x = 0;
            let mut chars = row_str.chars().peekable();
            while let Some(c) = chars.next() {
                if c.is_ascii_digit() {
                    let mut num_str = String::new();
                    num_str.push(c);
                    while let Some(&next_c) = chars.peek() {
                        if next_c.is_ascii_digit() {
                            num_str.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                    let num: i32 = num_str.parse().context("Invalid number in FEN")?;
                    x += num;
                } else {
                    let side = if c.is_uppercase() { Side::S0 } else { Side::S1 };
                    let unit = Unit::from_fen_char(c)
                        .ok_or_else(|| anyhow!("Invalid char in FEN: {}", c))?;
                    ensure!(x < GRID_LEN as i32, "FEN row {} is too long at char '{}'", y, c);
                    let loc = Loc::new(x, y as i32);
                    board.add_piece(Piece::new(unit, side, loc));
                    x += 1;
                }
            }
            ensure!(x == GRID_LEN as i32, "FEN row {} has incorrect length: {}, expected {}", y, x, GRID_LEN);
        }
        Ok(board)
    }

    pub fn get_valid_move_hexes(&self, piece_loc: Loc) -> Vec<Loc> {
        let piece = self.get_piece(&piece_loc).unwrap();
        let speed = piece.unit.stats().speed;
        let is_flying = piece.unit.stats().flying;
        self.bitboards.get_valid_moves(&piece_loc, piece.side, speed, is_flying).to_locs()
    }

    pub fn get_valid_moves(&self, piece_loc: Loc) -> Bitboard {
        let piece = self.get_piece(&piece_loc).unwrap();
        let speed = piece.unit.stats().speed;
        let is_flying = piece.unit.stats().flying;
        self.bitboards.get_valid_moves(&piece_loc, piece.side, speed, is_flying)
    }

    /// Returns a list of valid, empty locations where the given side can spawn units.
    pub fn get_spawn_locs(&self, side: Side, flying: bool) -> Vec<Loc> {
        self.bitboards.get_spawn_locs(side, flying).to_locs()
    }

}

impl Default for Board {
    fn default() -> Self {
        let map = Map::default();
        Self::from_fen(Self::START_FEN, map).expect("Failed to create default board")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::loc::Loc;
    use crate::core::units::Unit;

    #[test]
    fn test_board_pieces() {
        let map = Map::default();
        let mut board = Board::new(map);
        let loc = Loc::new(0, 0);
        let piece = Piece::new(Unit::Zombie, Side::S0, loc);
        board.add_piece(piece);
        assert_eq!(board.get_piece(&loc).unwrap().unit, Unit::Zombie);
        let removed = board.remove_piece(&loc).unwrap();
        assert_eq!(removed.unit, Unit::Zombie);
        assert!(board.get_piece(&loc).is_none());
    }

    #[test]
    fn test_board_fen_conversion() {
        let map = Map::default();
        let mut board = Board::new(map);

        board.add_piece(Piece::new(Unit::Zombie, Side::S0, Loc::new(0, 0)));
        board.add_piece(Piece::new(Unit::Zombie, Side::S1, Loc::new(1, 1)));
        board.add_piece(Piece::new(
            Unit::BasicNecromancer,
            Side::S0,
            Loc::new(2, 2),
        ));
        board.add_piece(Piece::new(
            Unit::BasicNecromancer,
            Side::S1,
            Loc::new(3, 3),
        ));

        let fen = board.to_fen();
        let new_board = Board::from_fen(&fen, map).unwrap();

        assert_eq!(board.pieces.len(), new_board.pieces.len());
        for (loc, piece) in &board.pieces {
            let new_piece = new_board.get_piece(loc).unwrap();
            assert_eq!(piece.unit, new_piece.unit);
            assert_eq!(piece.side, new_piece.side);
        }
    }

    #[test]
    fn test_fen_empty_board() {
        let map = Map::default();
        let board = Board::new(map);
        let fen = board.to_fen();
        let new_board = Board::from_fen(&fen, map).unwrap();
        assert!(new_board.pieces.is_empty());
    }

    #[test]
    fn test_fen_default_board() {
        let map = Map::default();
        let board = Board::new(map);
        let fen = board.to_fen();
        assert_eq!(fen, "10/10/10/10/10/10/10/10/10/10");
    }

    #[test]
    fn test_invalid_fen() {
        let map = Map::default();
        assert!(Board::from_fen("11/10", map).is_err());
        assert!(Board::from_fen("10/11", map).is_err());
        assert!(Board::from_fen("10/10/10/10/10/10/10/10/10/10/10", map).is_err());
        assert!(Board::from_fen("X/10", map).is_err());
        assert!(Board::from_fen("10/X", map).is_err());
    }

    #[test]
    fn test_board_reset() {
        let map = Map::default();
        let mut board = Board::new(map);

        board.add_piece(Piece::new(Unit::BasicNecromancer, Side::S0, Loc::new(0, 0)));
        board.add_piece(Piece::new(Unit::Zombie, Side::S0, Loc::new(0, 1)));
        board.add_piece(Piece::new(Unit::BasicNecromancer, Side::S1, Loc::new(1, 0)));
        board.add_piece(Piece::new(Unit::Zombie, Side::S1, Loc::new(1, 1)));

        board.reset();

        assert_eq!(board.state, BoardState::Reset0);
        assert_eq!(board.pieces.len(), 0);
        assert_eq!(board.reinforcements[Side::S0].contains(&Unit::BasicNecromancer), 1);
        assert_eq!(board.reinforcements[Side::S0].contains(&Unit::Zombie), 7);
        assert_eq!(board.reinforcements[Side::S1].contains(&Unit::BasicNecromancer), 1);
        assert_eq!(board.reinforcements[Side::S1].contains(&Unit::Zombie), 7);
    }
}

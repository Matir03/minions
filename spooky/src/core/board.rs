//! Board representation and rules

use std::{
    arch::is_aarch64_feature_detected, cell::RefCell, collections::HashMap
    // ops::Deref,
};
use hashbag::HashBag;
use anyhow::{Result, anyhow, bail, ensure};
use crate::core::side;

use super::{
    // bitboards::{Bitboard, BitboardOps},
    side::{Side, SideArray},
    units::{Unit, Attack},
    loc::{Loc, PATH_MAPS},
    action::BoardAction,
    spells::Spell,
    tech::TechState,
    map::{Map, Terrain},
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

/// Represents a piece on the board
#[derive(Debug, Clone)]
pub struct Piece {
    pub loc: Loc,
    pub side: Side,
    pub unit: Unit,
    pub modifiers: Modifiers,
    pub state: RefCell<PieceState>,
}

/// Represents a single Minions board
#[derive(Debug, Clone)]
pub struct Board {
    pub phase: Phase,
    pub pieces: HashMap<Loc, Piece>,
    pub reinforcements: SideArray<HashBag<Unit>>,
    pub spells: SideArray<HashBag<Spell>>,
    pub winner: Option<Side>,
    pub resetting: bool,
}

const GRAVEYARDS_TO_WIN: i32 = 8;

impl Board {
    const START_FEN: &str = "0/2ZZ6/1ZNZ6/1ZZ7/0/0/7zz1/6znz1/6zz2/0";

    /// Create a new empty board
    pub fn new() -> Self {
        Self {
            phase: Phase::Attack,
            pieces: HashMap::new(),
            reinforcements: SideArray::new(HashBag::new(), HashBag::new()),
            spells: SideArray::new(HashBag::new(), HashBag::new()),
            winner: None,
            resetting: false,
        }
    }

    pub fn get_piece(&self, loc: &Loc) -> Option<&Piece> {
        self.pieces.get(loc)
    }

    // pub fn get_mut_piece(&self, loc: &Loc) -> Option<&mut Piece> {
    //     self.pieces.get_mut(loc)
    // }

    // pub fn get_terrain(&self, _loc: &Loc) -> Option<Terrain> {
    //     todo!("refactor board to include terrain info");
    // }

    /// Add a piece to the board
    pub fn add_piece(&mut self, piece: Piece) {
        debug_assert!(!self.pieces.contains_key(&piece.loc));
        self.pieces.insert(piece.loc, piece);
    }

    /// Remove a piece from the board
    pub fn remove_piece(&mut self, loc: &Loc) -> Option<Piece> {
        self.pieces.remove(loc)
    }

    pub fn bounce_piece(&mut self, piece: Piece) {
        self.add_reinforcement(piece.unit, piece.side);
    }

    pub fn add_reinforcement(&mut self, unit: Unit, side: Side) {
        self.reinforcements[side].insert(unit);
    }

    pub fn can_move(&self, from_loc: &Loc, to_loc: &Loc, map: &Map, side_to_move: Side) -> Result<()> {
        // if let Some(piece) = self.get(to_loc) {
        //     ensure!(piece.side == side_to_move, "Cannot move to occupied square");            
        // }

        let piece = self.get_piece(from_loc)
            .ok_or(anyhow!("No piece to move from {from_loc}"))?;

        ensure!(piece.side == side_to_move, "Cannot move opponent's piece");
        ensure!(!piece.state.borrow().moved, "Cannot move piece twice");

        let delta = to_loc - from_loc;
        let stats = piece.unit.stats();
        let speed = stats.speed;

        ensure!(delta.length() <= speed, "Piece cannot move that far");

        if stats.flying { 
            return Ok(());
        }

        let paths = &PATH_MAPS[speed as usize][&delta];

        'outer: for path in paths {
            for delta in path {
                let loc = from_loc + delta;
                
                if !loc.in_bounds() { continue 'outer; }

                if let Some(piece) = self.get_piece(&loc) {
                    if piece.side != side_to_move { 
                        continue 'outer; 
                    }
                };

                if let Some(terrain) = map.get_terrain(&loc) {
                    if !terrain.allows(&piece.unit) { continue 'outer; }
                }
            }            

            return Ok(());
        }

        Err(anyhow!("No valid path from {from_loc} to {to_loc}"))
    }

    fn move_piece(&mut self, mut piece: Piece, to_loc: &Loc) {
        piece.loc = *to_loc;
        piece.state.borrow_mut().moved = true;

        self.add_piece(piece);
    }

    fn try_attack(&self, attacker_loc: Loc, target_loc: Loc, side_to_move: Side) -> Result<(bool, bool)> {
        let attacker= self.get_piece(&attacker_loc)
            .ok_or_else(|| anyhow!("No attacker at {attacker_loc}"))?;

        ensure!(attacker.side == side_to_move, "Cannot attack with opponent's piece");

        let attacker_stats = attacker.unit.stats();
        let mut attacker_state = attacker.state.borrow_mut();

        ensure!(!attacker_state.moved || !attacker_stats.lumbering,
            "Lumbering piece cannot move and attack on the same turn");
        
        ensure!(attacker_loc.dist(&target_loc) <= attacker_stats.range, 
            "Attack range not large enough");

        ensure!(attacker_state.attacks_used < attacker_stats.num_attacks,
            "Piece has already used all attacks");

        let target = self.get_piece(&target_loc)
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
                ensure!(!target_stats.necromancer, 
                    "Deathtouch cannot be used on necromancer");
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

    pub fn units_on_graveyards(&self, side: Side, map: &Map) -> i32 {
        map.spec().graveyards.iter()
            .filter(|loc| 
                self.get_piece(loc)
                    .map(|piece| piece.side == side)
                    .unwrap_or(false))
            .count() as i32
    }

    pub fn do_action(&mut self, 
            action: BoardAction, 
            money: &mut SideArray<i32>, 
            board_points: &mut SideArray<i32>, 
            tech_state: &TechState, 
            map: &Map,
            side_to_move: Side
        ) -> Result<()> {

        ensure!(!self.resetting, "Cannot do action on resetting board");

        if let Some(winner) = self.winner {
            match action {
                BoardAction::SaveUnit { unit } => {
                    if let Some(unit) = unit {
                        ensure!(!unit.stats().necromancer, "Cannot save necromancer");

                        ensure!(self.reinforcements[side_to_move].contains(&unit) > 0, 
                            "Cannot save unit not in reinforcements");

                        self.reinforcements[side_to_move].clear();
                        self.reinforcements[side_to_move].insert(unit);
                    } else {
                        self.reinforcements[side_to_move].clear();
                    }

                    if winner != side_to_move {
                        self.winner = None;
                    }
                }

                _ => bail!("Only SaveUnit action allowed on board yet to be restarted")
            }

            return Ok(());
        }
        
        match action {
            BoardAction::Move { from_loc, to_loc} => {
                ensure!(self.phase.is_attack(), "Cannot move in spawn phase");
                ensure!(self.get_piece(&to_loc).is_none(), "Cannot move to occupied square");

                self.can_move(&from_loc, &to_loc, map, side_to_move)?;

                let piece = self.remove_piece(&from_loc).unwrap();
                self.move_piece(piece, &to_loc);
            }
            BoardAction::MoveCyclic { locs } => {
                ensure!(self.phase.is_attack(), "Cannot move in spawn phase");

                let num_locs = locs.len();
                ensure!(num_locs >= 2, "MoveCyclic must have at least 2 locations");

                // Verify all moves are valid first
                for i in 0..num_locs {
                    let from_loc = &locs[i];
                    let to_loc = &locs[(i + 1) % num_locs];
                    self.can_move(from_loc, to_loc, map, side_to_move)?;
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
                ensure!(self.phase.is_attack(), "Cannot attack in spawn phase");

                let (remove, bounce) = 
                    self.try_attack(attacker_loc, target_loc, side_to_move)?;

                if !remove { return Ok(()); }

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
                ensure!(self.phase.is_spawn(), "Cannot buy in attack phase");

                ensure!(tech_state.can_buy(side_to_move, unit), "Unit not teched to");

                let cost = unit.stats().cost;
                ensure!(money[side_to_move] >= cost, "Not enough money");

                money[side_to_move] -= cost;

                self.add_reinforcement(unit, side_to_move);
            }
            BoardAction::Spawn { spawn_loc, unit } => {
                ensure!(self.phase.is_spawn(), "Cannot spawn in attack phase");

                ensure!(self.get_piece(&spawn_loc).is_none(), "Can't spawn on occupied square");

                if let Some(terrain) = map.get_terrain(&spawn_loc) {
                    ensure!(terrain.allows(&unit), "Terrain does not allow spawn");
                }

                let has_spawner = spawn_loc.neighbors().iter().any(|neighbor| 
                    self.get_piece(neighbor)
                        .map_or(false, 
                            |piece| 
                            piece.side == side_to_move && 
                            piece.unit.stats().spawn &&
                            !piece.state.borrow().exhausted
                        )
                    );

                ensure!(has_spawner, "No spawner around spawn location");

                let num_units = self.reinforcements[side_to_move].remove(&unit);
                ensure!(num_units > 0, "Can't spawn unit not in reinforcements");

                let piece = Piece {
                    loc: spawn_loc,
                    side: side_to_move,
                    unit,
                    modifiers: Modifiers::default(),
                    state: PieceState::spawned().into(),
                };

                self.add_piece(piece);
            }
            BoardAction::Cast { spell_cast } => {
                spell_cast.cast(self, side_to_move)?;
            }
            BoardAction::Discard { spell } => {
                let count = self.spells[side_to_move].remove(&spell);
                ensure!(count > 0, "Cannot discard unowned spell");
            }
            BoardAction::EndPhase => {
                self.phase = Phase::Spawn;
            }
            BoardAction::Resign => {
                self.lose(side_to_move, board_points);
            }
            BoardAction::SaveUnit { .. } => {
                bail!("Cannot save unit during game")
            }
        }

        Ok(())
    }

    pub fn end_turn(&mut self, 
            money: &mut SideArray<i32>,
            map: &Map,
            board_points: &mut SideArray<i32>,
            side_to_move: Side, 
        ) -> Result<()> {

        let income = self.units_on_graveyards(side_to_move, map) + 3;
        money[side_to_move] += income;

        if self.winner.is_none() {
            let enemies_on_graveyards = self.units_on_graveyards(!side_to_move, map);

            if enemies_on_graveyards >= GRAVEYARDS_TO_WIN  {
                self.lose(side_to_move, board_points);
            }
        }

        self.phase = Phase::Attack;

        if self.resetting {
            self.reset();
            self.resetting = false;

            return Ok(());
        } 

        if self.winner == Some(side_to_move) {
            ensure!(self.reinforcements[side_to_move].iter().count() <= 1, 
                "Must choose a unit to save before ending winning turn");
        }

        for piece in self.pieces.values() {
            piece.state.borrow_mut().reset();
        }

        Ok(())
    }

    pub fn assign_spell(&mut self, spell: Spell, side: Side) -> Result<()> {
        self.spells[side].insert(spell);
        Ok(())
    }


    pub fn win(&mut self, side: Side, board_points: &mut SideArray<i32>) {
        self.winner = Some(side);
        board_points[side] += 1;
        self.reset();
    }

    pub fn lose(&mut self, side: Side, board_points: &mut SideArray<i32>) {
        self.winner = Some(!side);
        board_points[!side] += 1;
        self.resetting = true;
    }

    fn clear(&mut self) {
        for (loc, piece) in self.pieces.iter() {
            self.reinforcements[piece.side].insert(piece.unit);
        }

        self.pieces.clear();
    }

    fn reset(&mut self) {
        self.clear();

        let reinforcements = self.reinforcements.clone();

        self.pieces = Board::default().pieces;
        self.reinforcements = reinforcements;
    }

    /// Convert board state to FEN notation
    pub fn to_fen(&self) -> String {
        let mut fen = String::new();
        let mut empty_count = 0;
        
        for y in 0..10 {
            if y > 0 {
                fen.push('/');
            }
            
            for x in 0..10 {
                let loc = Loc { y, x };
                if let Some(piece) = self.get_piece(&loc) {
                    if empty_count > 0 {
                        if empty_count == 10 {
                            fen.push('0');
                        } else {
                            fen.push(char::from_digit(empty_count as u32, 10).unwrap());
                        }
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
                if empty_count == 10 {
                    fen.push('0');
                } else {
                    fen.push(char::from_digit(empty_count as u32, 10).unwrap());
                }
                empty_count = 0;
            }
        }
        
        fen
    }

    /// Create a board from FEN notation
    pub fn from_fen(fen: &str) -> Result<Self> {
        let mut board = Board::new();
        let mut y = 0;
        let mut x = 0;

        for c in fen.chars() {
            match c {
                '/' => {
                    if x != 10 {
                        bail!("Invalid FEN: wrong number of squares in y {}", y);
                    }
                    y += 1;
                    x = 0;
                }
                '0'..='9' => {
                    let empty = if c == '0' { 10 } else { c.to_digit(10).unwrap() as usize };
                    x += empty;
                }
                'A'..='Z' | 'a'..='z' => {
                    if x >= 10 || y >= 10 {
                        bail!("Invalid FEN: piece position out of bounds");
                    }

                    let side = if c.is_uppercase() { Side::S0 } else { Side::S1 };
                    if let Some(unit) = Unit::from_fen_char(c.to_ascii_uppercase()) {
                        board.add_piece(Piece {
                            loc: Loc { 
                                y: y as i32, 
                                x: x as i32 
                            },
                            side,
                            unit,
                            modifiers: Modifiers::default(),
                            state: PieceState::default().into(),
                        });
                        x += 1;
                    } else {
                        bail!("Invalid FEN: unknown piece '{}'", c);
                    }
                }
                _ => bail!("Invalid FEN character: '{}'", c),
            }
        }

        if y != 9 || x != 10 {
            bail!("Invalid FEN: wrong number of ys or xumns");
        }

        Ok(board)
    }
}

impl Default for Board {
    fn default() -> Self {
        Board::from_fen(Self::START_FEN).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_board_pieces() {
        let mut board = Board::new();
        let piece = Piece {
            loc: Loc { y: 0, x: 0 },
            side: Side::S0,
            unit: Unit::Zombie,
            modifiers: Modifiers::default(),
            state: PieceState::default().into(),
        };
        board.add_piece(piece);

        assert!(board.get_piece(&Loc { y: 0, x: 0 }).is_some());
        assert!(board.get_piece(&Loc { y: 0, x: 1 }).is_none());
    }

    #[test]
    fn test_board_fen_conversion() {
        let mut board = Board::new();
        
        // Add a zombie at (0,0) for Side 0
        board.add_piece(Piece {
            loc: Loc { y: 0, x: 0 },
            side: Side::S0,
            unit: Unit::Zombie,
            modifiers: Modifiers::default(),
            state: PieceState::default().into(),
        });

        // Add an initiate at (0,9) for Side 1
        board.add_piece(Piece {
            loc: Loc { y: 0, x: 9 },
            side: Side::S1,
            unit: Unit::Initiate,
            modifiers: Modifiers::default(),
            state: PieceState::default().into(),
        });

        assert_eq!(board.to_fen(), "Z8i/0/0/0/0/0/0/0/0/0");

        let board2 = Board::from_fen(&board.to_fen()).unwrap();
        assert_eq!(board2.to_fen(), board.to_fen());
    }

    #[test]
    fn test_fen_empty_board() {
        let board = Board::new();
        assert_eq!(board.to_fen(), "0/0/0/0/0/0/0/0/0/0");

        let board2 = Board::from_fen("0/0/0/0/0/0/0/0/0/0").unwrap();
        assert_eq!(board2.to_fen(), "0/0/0/0/0/0/0/0/0/0");
    }

    #[test]
    fn test_fen_default_board() {
        let board = Board::default();
        assert_eq!(board.to_fen(), "0/2ZZ6/1ZNZ6/1ZZ7/0/0/7zz1/6znz1/6zz2/0");
    }

    #[test]
    fn test_invalid_fen() {
        // Test invalid piece character
        assert!(Board::from_fen("X9/0/0/0/0/0/0/0/0/0").is_err());

        // Test wrong number of ys
        assert!(Board::from_fen("0/0/0/0/0/0/0/0/0").is_err());

        // Test wrong number of squares in y
        assert!(Board::from_fen("Z8/0/0/0/0/0/0/0/0/0").is_err());
    }
}

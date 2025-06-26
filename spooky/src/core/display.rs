use std::fmt;
use colored::Colorize;

use super::{
    board::{Board, Piece, PieceState},
    game::GameState,
    loc::Loc,
    map::{Terrain, TileType},
    side::Side,
    tech::{Tech, TechState},
    units::Unit,
};

impl<'a> fmt::Display for GameState<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f)?;
        writeln!(f, "Current Turn: {}", self.side_to_move)?;
        writeln!(
            f,
            "Points: {} | {}",
            self.board_points[Side::S0].to_string().bright_blue(),
            self.board_points[Side::S1].to_string().bright_red()
        )?;
        writeln!(
            f,
            "Money: {} | {}",
            self.money[Side::S0].to_string().bright_blue(),
            self.money[Side::S1].to_string().bright_red()
        )?;
        writeln!(f)?;

        writeln!(f, "Tech State:")?;
        write!(f, "{}", self.tech_state)?;

        for (i, board) in self.boards.iter().enumerate() {
            writeln!(f)?;
            writeln!(f, "Board {}:", i)?;
            write!(f, "{}", board)?;

            let mut s0_units: Vec<_> = board.pieces.values().filter(|p| p.side == Side::S0).collect();
            let mut s1_units: Vec<_> = board.pieces.values().filter(|p| p.side == Side::S1).collect();
            s0_units.sort_by_key(|p| p.loc);
            s1_units.sort_by_key(|p| p.loc);

            if !s0_units.is_empty() {
                writeln!(f, "{}", "Blue units:".bright_blue())?;
                for piece in s0_units {
                    let state = piece.state;
                    let defense = piece.unit.stats().defense - state.damage_taken;
                    let status = if state.can_act() {
                        "Ready".green()
                    } else {
                        "Done".dimmed()
                    };
                    writeln!(
                        f,
                        "  - {} at {}: {}/{} HP ({})",
                        piece.unit.to_fen_char(),
                        piece.loc,
                        defense,
                        piece.unit.stats().defense,
                        status
                    )?;
                }
            }

            if !s1_units.is_empty() {
                writeln!(f, "{}", "Red units:".bright_red())?;
                for piece in s1_units {
                    let state = piece.state;
                    let defense = piece.unit.stats().defense - state.damage_taken;
                    let status = if state.can_act() {
                        "Ready".green()
                    } else {
                        "Done".dimmed()
                    };
                    writeln!(
                        f,
                        "  - {} at {}: {}/{} HP ({})",
                        piece.unit.to_fen_char(),
                        piece.loc,
                        defense,
                        piece.unit.stats().defense,
                        status
                    )?;
                }
            }
        }

        Ok(())
    }
}

impl<'a> fmt::Display for Board<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Print column numbers with proper spacing for hex grid
        write!(f, "    ")?;
        for x in 0..10 {
            write!(f, " {} ", x)?;
        }
        writeln!(f)?;

        // Top border with proper hex spacing
        write!(f, "   ")?;
        writeln!(f, "{}", "─".repeat(32))?;

        for y in 0..10 {
            // Reversed to match game coordinates
            // Add proper indentation for hex grid
            let indent = y as usize;
            write!(f, "{:2} {}", y, " ".repeat(indent))?;
            write!(f, "\\")?;

            for x in 0..10 {
                let loc = Loc::new(x, y);
                if let Ok(piece) = self.get_piece(&loc) {
                    write!(f, " {} ", piece)?;
                } else {
                    let tile_type = self.map.spec().tiles.get(&loc).unwrap_or(&TileType::Ground);
                    let terrain_char = match tile_type {
                        TileType::Ground => ".".dimmed(),
                        TileType::Graveyard => "G".white().bold(),
                        TileType::NativeTerrain(terrain) => {
                            let symbol = match terrain {
                                Terrain::Flood => "~",
                                Terrain::Earthquake => "E",
                                Terrain::Whirlwind => "W",
                                Terrain::Firestorm => "F",
                            };
                            symbol.yellow()
                        }
                    };
                    write!(f, " {} ", terrain_char)?;
                }
            }
            writeln!(f, " \\")?;
        }

        // Bottom border with proper hex spacing
        write!(f, "    {}", " ".repeat(9))?;
        writeln!(f, "{}", "─".repeat(32))?;
        Ok(())
    }
}

impl fmt::Display for Piece {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let symbol = self.unit.to_fen_char().to_string();
        let state = self.state;

        let mut colored_symbol = match self.side {
            Side::S0 => symbol.bright_blue(),
            Side::S1 => symbol.bright_red(),
        };

        if !state.can_act() {
            colored_symbol = colored_symbol.dimmed();
        }

        write!(f, "{}", colored_symbol)
    }
}

impl fmt::Display for Side {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Side::S0 => write!(f, "{}", "Blue".bright_blue()),
            Side::S1 => write!(f, "{}", "Red".bright_red()),
        }
    }
}

impl fmt::Display for TechState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", "Blue Tech: ".bright_blue())?;
        for (i, tech) in self.status[Side::S0].iter().enumerate() {
            write!(f, "{:?} ", tech)?;
        }
        writeln!(f)?;

        write!(f, "{}", "Red Tech: ".bright_red())?;
        for (i, tech) in self.status[Side::S1].iter().enumerate() {
            write!(f, "{:?} ", tech)?;
        }
        writeln!(f)?;

        Ok(())
    }
}

use std::fmt;
use colored::Colorize;

use super::{
    game::{Game, GameState},
    board::{Board, Piece, PieceState},
    side::Side,
    units::Unit,
    loc::Loc,
    tech::{Tech, TechState},
    map::Terrain,
};

impl fmt::Display for Game<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "=== Minions Game ===")?;
        writeln!(f, "Current Side: {}", self.state.side_to_move)?;
        writeln!(f, "Points: S0: {} | S1: {}", 
            self.state.board_points[Side::S0],
            self.state.board_points[Side::S1])?;
        writeln!(f, "Money: S0: {} | S1: {}", 
            self.state.money[Side::S0],
            self.state.money[Side::S1])?;
        writeln!(f)?;

        writeln!(f, "Tech State:")?;
        write!(f, "{}", self.state.tech_state)?;

        for (i, board) in self.state.boards.iter().enumerate() {
            writeln!(f, "Board {}:", i)?;
            write!(f, "{}", board)?;
            writeln!(f)?;
        }

        Ok(())
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Print column numbers with proper spacing for hex grid
        write!(f, "    ")?;
        for x in 0..10 {
            write!(f, "{} ", x)?;
        }
        writeln!(f)?;

        // Top border with proper hex spacing
        write!(f, "    ")?;
        writeln!(f, "{}", "─".repeat(32))?;

        for y in 0..10 {  // Reversed to match game coordinates
            // Add proper indentation for hex grid
            let indent = y as usize;
            write!(f, "{:2} {}", y, " ".repeat(indent))?;
            write!(f, "│")?;
            
            for x in 0..10 {
                let loc = Loc::new(x, y);
                if let Some(piece) = self.get_piece(&loc) {
                    write!(f, " {} ", piece)?;
                } else {
                    write!(f, " · ")?;  // Using middle dot for empty spaces
                }
            }
            writeln!(f, " │")?;
        }

        // Bottom border with proper hex spacing
        write!(f, "    ")?;
        writeln!(f, "{}", "─".repeat(32))?;
        Ok(())
    }
}

impl fmt::Display for Piece {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let symbol = self.unit.to_fen_char().to_string();

        let colored_symbol = match self.side {
            Side::S0 => symbol.bright_blue(),
            Side::S1 => symbol.bright_red(),
        };

        write!(f, "{}", colored_symbol)
    }
}

impl fmt::Display for Side {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Side::S0 => write!(f, "{}", "Player 1".bright_blue()),
            Side::S1 => write!(f, "{}", "Player 2".bright_red()),
        }
    }
}

impl fmt::Display for TechState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "S0 Tech:")?;
        for (i, tech) in self.status[Side::S0].iter().enumerate() {
            write!(f, "{:?} ", tech)?;
            if (i + 1) % 5 == 0 {
                writeln!(f)?;
            }
        }
        writeln!(f)?;

        writeln!(f, "S1 Tech:")?;
        for (i, tech) in self.status[Side::S1].iter().enumerate() {
            write!(f, "{:?} ", tech)?;
            if (i + 1) % 5 == 0 {
                writeln!(f)?;
            }
        }
        writeln!(f)?;
        Ok(())
    }
}

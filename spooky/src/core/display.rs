use std::fmt;
use colored::Colorize;

use super::{
    game::GameState,
    board::{Board, Piece, PieceState},
    side::Side,
    units::Unit,
    loc::Loc,
    tech::{Tech, TechState},
    map::Terrain,
};

impl fmt::Display for GameState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // writeln!(f, "=== Minions Game ===")?;
        writeln!(f)?;
        writeln!(f, "Current Turn: {}", self.side_to_move)?;
        writeln!(f, "Points: {} | {}", 
            self.board_points[Side::S0].to_string().bright_blue(),
            self.board_points[Side::S1].to_string().bright_red())?;
        writeln!(f, "Money: {} | {}", 
            self.money[Side::S0].to_string().bright_blue(),
            self.money[Side::S1].to_string().bright_red())?;
        writeln!(f)?;

        writeln!(f, "Tech State:")?;
        write!(f, "{}", self.tech_state)?;

        for (i, board) in self.boards.iter().enumerate() {
            writeln!(f)?;
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
            write!(f, " {} ", x)?;
        }
        writeln!(f)?;

        // Top border with proper hex spacing
        write!(f, "   ")?;
        writeln!(f, "{}", "─".repeat(32))?;

        for y in 0..10 {  // Reversed to match game coordinates
            // Add proper indentation for hex grid
            let indent = y as usize;
            write!(f, "{:2} {}", y, " ".repeat(indent))?;
            write!(f, "\\")?;
            
            for x in 0..10 {
                let loc = Loc::new(x, y);
                if let Some(piece) = self.get_piece(&loc) {
                    write!(f, " {} ", piece)?;
                } else {
                    write!(f, " · ")?;  // Using middle dot for empty spaces
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

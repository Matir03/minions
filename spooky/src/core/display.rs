use std::fmt::{self, Display, Formatter, Result};
use colored::{Color, ColoredString, Colorize};

use crate::core::{tech::NUM_TECHS, SideArray};

use super::{
    board::{Board, Piece, PieceState},
    game::GameState,
    loc::Loc,
    map::{Terrain, TileType},
    side::Side,
    tech::{Tech, TechState, Techline, TechStatus},
    units::Unit,
};

impl<'a> Display for GameState<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f)?;
        writeln!(f, "Current Turn: {}", self.side_to_move)?;
        writeln!(
            f,
            "Points: {} | {}",
            self.board_points[Side::Yellow].to_string().yellow(),
            self.board_points[Side::Blue].to_string().blue()
        )?;
        writeln!(
            f,
            "Money: {} | {}",
            self.money[Side::Yellow].to_string().yellow(),
            self.money[Side::Blue].to_string().blue()
        )?;
        writeln!(f)?;

        fmt_techline(&self.config.techline, &self.tech_state, f)?;

        for (i, board) in self.boards.iter().enumerate() {
            writeln!(f)?;
            // writeln!(f, "Board {}:", i)?;
            write!(f, "{}", board)?;

            // let mut s0_units: Vec<_> = board.pieces.values().filter(|p| p.side == Side::Yellow).collect();
            // let mut s1_units: Vec<_> = board.pieces.values().filter(|p| p.side == Side::Blue).collect();
            // s0_units.sort_by_key(|p| p.loc);
            // s1_units.sort_by_key(|p| p.loc);

            // if !s0_units.is_empty() {
            //     writeln!(f, "{}", "Yellow units:".bright_yellow())?;
            //     for piece in s0_units {
            //         let state = piece.state;
            //         let defense = piece.unit.stats().defense - state.damage_taken;
            //         let status = if state.can_act() {
            //             "Ready".green()
            //         } else {
            //             "Done".dimmed()
            //         };
            //         writeln!(
            //             f,
            //             "  - {} at {}: {}/{} HP ({})",
            //             piece.unit.to_fen_char(),
            //             piece.loc,
            //             defense,
            //             piece.unit.stats().defense,
            //             status
            //         )?;
            //     }
            // }

            // if !s1_units.is_empty() {
            //     writeln!(f, "{}", "Blue units:".bright_blue())?;
            //     for piece in s1_units {
            //         let state = piece.state;
            //         let defense = piece.unit.stats().defense - state.damage_taken;
            //         let status = if state.can_act() {
            //             "Ready".green()
            //         } else {
            //             "Done".dimmed()
            //         };
            //         writeln!(
            //             f,
            //             "  - {} at {}: {}/{} HP ({})",
            //             piece.unit.to_fen_char(),
            //             piece.loc,
            //             defense,
            //             piece.unit.stats().defense,
            //             status
            //         )?;
            //     }
            // }
        }

        Ok(())
    }
}

impl<'a> Display for Board<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        // Print column numbers with proper spacing for hex grid
        // a to j
        write!(f, "    ")?;
        for x in 0..10u8 {
            write!(f, " {} ", (x + b'a') as char)?;
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
                let tile_type = self.map.spec().tiles.get(&loc).unwrap();

                let symbol = if let Ok(piece) = self.get_piece(&loc) {
                    format_for(tile_type, piece.to_string().as_str())
                } else {
                    format_for(tile_type, tile_type.to_display_char().to_string().as_str())
                };

                write!(f, "{}", symbol)?;
            }
            writeln!(f, " \\")?;
        }

        // Bottom border with proper hex spacing
        write!(f, "    {}", " ".repeat(9))?;
        writeln!(f, "{}", "─".repeat(32))?;
        Ok(())
    }
}

fn format_for(tile_type: &TileType, symbol: impl Colorize + Display) -> ColoredString {
    let spaced_symbol = format!(" {} ", symbol);
    match tile_type {
        TileType::Ground => spaced_symbol.on_green(),
        TileType::Graveyard => spaced_symbol.bold().on_black(),
        TileType::NativeTerrain(t) => spaced_symbol.on_cyan(),
    }
}

impl Display for Piece {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let symbol = self.unit.to_fen_char().to_string();
        let state = self.state;

        let mut colored_symbol = match self.side {
            Side::Yellow => symbol.yellow().dimmed(),
            Side::Blue => symbol.blue().dimmed(),
        };

        write!(f, "{}", colored_symbol)
    }
}

impl Display for Side {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Side::Yellow => write!(f, "{}", "Yellow".yellow()),
            Side::Blue => write!(f, "{}", "Blue".blue()),
        }
    }
}

fn fmt_techline(techline: &Techline, state: &TechState, f: &mut Formatter<'_>) -> Result {
    write!(f, "{}", "Yellow  :".yellow())?;
    for (i, tech) in state.status[Side::Yellow].iter().enumerate() {
        write!(f, "{}", tech.to_string().yellow())?;
    }
    writeln!(f)?;

    write!(f, "Techline:")?;
    for (i, tech) in techline.techs.iter().enumerate() {
        let cur_color = if state.status[Side::Yellow][i] == TechStatus::Acquired {
            Color::Yellow
        } else if state.status[Side::Blue][i] == TechStatus::Acquired {
            Color::Blue
        } else {
            Color::White
        };

        let tech_char = match tech {
            Tech::UnitTech(unit) => unit.to_fen_char(),
            Tech::Copycat => '🐱',
            Tech::Thaumaturgy => '✨',
            Tech::Metamagic => '🔄',
        };

        write!(f, " {} ", tech_char.to_string().color(cur_color))?;
    }
    writeln!(f)?;

    write!(f, "{}", "Blue    :".blue())?;
    for (i, tech) in state.status[Side::Blue].iter().enumerate() {
        write!(f, "{}", tech.to_string().blue())?;
    }
    writeln!(f)?;

    Ok(())
}

impl Display for TechStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let s = match self {
            TechStatus::Locked   => "   ",
            TechStatus::Unlocked => " ■ ",
            TechStatus::Acquired => " ■■",
        };
        write!(f, "{}", s)
    }
}

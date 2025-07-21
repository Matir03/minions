use crate::core::convert::{FromIndex, ToIndex};
use crate::core::tech::{TechState, TechStatus, Techline};
use crate::core::{Board, GameConfig, GameState, Side, SideArray};
use anyhow::{anyhow, bail, ensure, Context, Result};

impl<'a> GameState<'a> {
    /// Convert state to FEN notation
    /// <money> <board_points> <tech_state> <board_fen> ... <board_fen> <side_to_move> <turn_num>
    /// - Each field is separated by a single space.
    /// - Boards and money are separated by '|'.
    pub fn to_fen(&self) -> Result<String> {
        let mut fen = String::new();

        // Money
        fen.push_str(&format!(
            "{}|{}",
            self.money.get(Side::Yellow)?.to_string().trim(),
            self.money.get(Side::Blue)?.to_string().trim()
        ));

        // Board points
        fen.push(' ');
        fen.push_str(&format!(
            "{}|{}",
            self.board_points.get(Side::Yellow)?.to_string().trim(),
            self.board_points.get(Side::Blue)?.to_string().trim()
        ));

        // Tech state
        fen.push(' ');
        fen.push_str(&self.tech_state.to_fen().trim());

        // Board states
        for board in self.boards.iter() {
            fen.push(' ');
            fen.push_str(&board.to_fen().trim());
        }

        // Side to move
        fen.push(' ');
        fen.push_str(&self.side_to_move.to_index()?.to_string().trim());

        // Turn number
        fen.push(' ');
        fen.push_str(&self.ply.to_string().trim());

        Ok(fen)
    }

    /// Parse state from FEN notation
    /// <money> <board_points> <tech_state> <board_fen> ... <board_fen> <side_to_move> <turn_num>
    /// - Each field is separated by a single space.
    /// - Boards and money are separated by '|'.
    pub fn from_fen(fen: &str, config: &'a GameConfig) -> Result<Self> {
        let mut parts = fen.split_whitespace();

        // Parse money
        let money_parts: Vec<_> = parts.next().context("Missing money")?.split('|').collect();
        ensure!(money_parts.len() == 2, "Invalid money format");

        let money = SideArray::new(
            money_parts[0]
                .parse()
                .context("Invalid money value for side 0")?,
            money_parts[1]
                .parse()
                .context("Invalid money value for side 1")?,
        );

        // Parse board points
        let board_points_parts: Vec<_> = parts
            .next()
            .context("Missing board points")?
            .split('|')
            .collect();
        ensure!(board_points_parts.len() == 2, "Invalid board points format");

        let board_points = SideArray::new(
            board_points_parts[0]
                .parse()
                .context("Invalid board points value for side 0")?,
            board_points_parts[1]
                .parse()
                .context("Invalid board points value for side 1")?,
        );

        // Parse tech state
        let tech_fen = parts.next().context("Missing tech state")?;
        let tech_state = TechState::from_fen(tech_fen, &config.techline)?;

        // Parse board states
        let mut boards = Vec::new();
        for map in &config.maps {
            let board_fen = parts.next().context("Missing board state")?;
            boards.push(Board::from_fen(board_fen, map)?);
        }

        ensure!(
            boards.len() == config.num_boards,
            "Invalid number of boards"
        );

        // Parse side to move
        let side_idx = parts
            .next()
            .context("Missing side to move")?
            .parse::<usize>()
            .context("Invalid side to move")?;
        let side_to_move = Side::from_index(side_idx)?;

        // Parse turn number
        let turn_num = parts
            .next()
            .context("Missing turn number")?
            .parse::<i32>()
            .context("Invalid turn number")?;

        // Parse winner
        // let winner_char = parts
        //     .next()
        //     .context("Missing winner")?
        //     .chars()
        //     .next()
        //     .context("Empty winner string")?;
        // let winner = match winner_char {
        //     '_' => None,
        //     '0' => Some(Side::Yellow),
        //     '1' => Some(Side::Blue),
        //     _ => bail!("Invalid winner char"),
        // };

        Ok(Self {
            config,
            side_to_move,
            ply: turn_num,
            boards,
            board_points,
            tech_state,
            money,
            winner: None,
        })
    }
}

impl TechState {
    pub fn to_fen(&self) -> String {
        let mut fen = String::new();

        for (i, side_techs) in self.status.iter().enumerate() {
            if i > 0 {
                fen.push('|');
            }
            for status in side_techs {
                fen.push(match status {
                    TechStatus::Locked => 'L',
                    TechStatus::Unlocked => 'U',
                    TechStatus::Acquired => 'A',
                });
            }
        }

        fen
    }

    pub fn from_fen(fen: &str, techline: &Techline) -> Result<Self> {
        let mut state = Self::new();
        let num_techs = techline.techs.len();

        let side_strs = fen.split('|');

        ensure!(side_strs.clone().count() == 2);
        for (side_index, side_str) in side_strs.enumerate() {
            ensure!(side_str.len() == num_techs);

            let side = Side::from_index(side_index)?;
            for (i, c) in side_str.chars().enumerate() {
                state.status[side][i] = match c {
                    'L' => {
                        // Record the first 'L' as the unlock_index, but continue parsing to fill status array.
                        // This might be refined later if multiple 'L's are disallowed or handled differently.
                        if state.status[side]
                            .iter()
                            .take(i)
                            .all(|s| *s != TechStatus::Locked)
                        {
                            // only set if this is the first L
                            state.unlock_index[side] = i;
                        }
                        TechStatus::Locked
                    }
                    'U' => TechStatus::Unlocked,
                    'A' => {
                        state.acquired_techs[side].insert(techline[i]);
                        TechStatus::Acquired
                    }
                    _ => bail!("Invalid tech status"),
                };
            }
        }

        state.unlock_index.values = state.status.values.map(|side_techs| {
            side_techs
                .iter()
                .position(|&status| status == TechStatus::Locked)
                .unwrap_or(num_techs)
        });

        state.acquired_techs.values = state.status.values.map(|side_techs| {
            side_techs
                .iter()
                .enumerate()
                .filter(|(_, &status)| status == TechStatus::Acquired)
                .map(|(i, _)| techline[i])
                .collect()
        });

        Ok(state)
    }
}

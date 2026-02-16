use crate::core::convert::{FromIndex, ToIndex};
use crate::core::game::generate_game_id;
use crate::core::tech::{Tech, TechState, TechStatus, Techline};
use crate::core::{Board, GameConfig, GameState, Map, Side, SideArray, Unit};
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
            game_id: generate_game_id(),
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

impl GameConfig {
    /// Convert config to FEN notation
    pub fn to_fen(&self) -> Result<String> {
        let mut fen = String::new();

        fen.push_str(&self.points_to_win.to_string());
        fen.push(' ');

        fen.push_str(
            &self
                .maps
                .iter()
                .map(|m| m.to_fen())
                .collect::<Result<Vec<_>>>()?
                .join(","),
        );
        fen.push(' ');

        fen.push_str(
            &self
                .techline
                .techs
                .iter()
                .map(|t| t.to_fen())
                .collect::<Result<Vec<_>>>()?
                .join(","),
        );

        Ok(fen)
    }

    /// Parse config from FEN notation
    pub fn from_fen(fen: &str) -> Result<Self> {
        let mut parts = fen.split_whitespace();

        // Parse points to win
        let points_to_win = parts
            .next()
            .context("Missing points to win")?
            .parse::<i32>()
            .context("Invalid points to win")?;

        // Parse maps
        let maps = parts
            .next()
            .context("Missing maps")?
            .split(',')
            .map(Map::from_fen)
            .collect::<Result<Vec<_>>>()?;

        // Parse techline
        let techs = parts
            .next()
            .context("Missing techline")?
            .split(',')
            .map(Tech::from_fen)
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            num_boards: maps.len(),
            points_to_win,
            maps,
            techline: Techline { techs },
        })
    }
}

impl Map {
    pub fn to_fen(&self) -> Result<String> {
        match self {
            Map::AllLand => Ok("AllLand".to_string()),
            Map::BlackenedShores => Ok("BlackenedShores".to_string()),
            Map::MidnightLake => Ok("MidnightLake".to_string()),
        }
    }

    pub fn from_fen(fen: &str) -> Result<Self> {
        match fen {
            "AllLand" => Ok(Map::AllLand),
            "BlackenedShores" => Ok(Map::BlackenedShores),
            "MidnightLake" => Ok(Map::MidnightLake),
            _ => bail!("Invalid map"),
        }
    }
}

impl Tech {
    pub fn to_fen(&self) -> Result<String> {
        match self {
            Tech::Copycat => Ok("1".to_string()),
            Tech::Thaumaturgy => Ok("2".to_string()),
            Tech::Metamagic => Ok("3".to_string()),
            Tech::UnitTech(unit) => Ok(unit.to_fen_char().to_string()),
        }
    }

    pub fn from_fen(fen: &str) -> Result<Self> {
        let c = fen.chars().next().context("Invalid tech")?;
        let tech = match c {
            '1' => Self::Copycat,
            '2' => Self::Thaumaturgy,
            '3' => Self::Metamagic,
            _ => Self::UnitTech(Unit::from_fen_char(c).context("Invalid tech")?),
        };

        Ok(tech)
    }
}

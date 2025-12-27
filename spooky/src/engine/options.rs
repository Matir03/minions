/// Configuration options for the engine
use anyhow::{bail, Result};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeuristicType {
    Naive,
    Random,
}

impl FromStr for HeuristicType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "naive" => Ok(HeuristicType::Naive),
            "random" => Ok(HeuristicType::Random),
            _ => bail!("Unknown heuristic type: {}", s),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EngineOptions {
    /// Whether spells are enabled in the game
    pub spells_enabled: bool,
    /// Whether strict mode is enabled
    pub strict_mode: bool,
    /// The heuristic to use
    pub heuristic: HeuristicType,
}

impl EngineOptions {
    /// Create new engine options with custom parameters
    pub fn new(spells_enabled: bool, strict_mode: bool, heuristic: HeuristicType) -> Self {
        Self {
            spells_enabled,
            strict_mode,
            heuristic,
        }
    }

    /// Set whether spells are enabled
    pub fn set_option(&mut self, name: &str, value: &str) -> Result<()> {
        match name {
            "spells" => self.spells_enabled = value.parse()?,
            "strictmode" => self.strict_mode = value.parse()?,
            "heuristic" => self.heuristic = value.parse()?,
            _ => bail!("Unknown option: {}", name),
        }

        Ok(())
    }
}

impl Default for EngineOptions {
    fn default() -> Self {
        Self {
            spells_enabled: false,
            strict_mode: true,
            heuristic: HeuristicType::Naive,
        }
    }
}

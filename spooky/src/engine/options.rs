/// Configuration options for the engine
use anyhow::{bail, Result};

#[derive(Debug, Clone)]
pub struct EngineOptions {
    /// Whether spells are enabled in the game
    pub spells_enabled: bool,
    /// Whether strict mode is enabled
    pub strict_mode: bool,
}

impl EngineOptions {
    /// Create new engine options with custom parameters
    pub fn new(spells_enabled: bool, strict_mode: bool) -> Self {
        Self {
            spells_enabled,
            strict_mode,
        }
    }

    /// Set whether spells are enabled
    pub fn set_option(&mut self, name: &str, value: &str) -> Result<()> {
        match name {
            "spells" => self.spells_enabled = value.parse()?,
            "strictmode" => self.strict_mode = value.parse()?,
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
        }
    }
}

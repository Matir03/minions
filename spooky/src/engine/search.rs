use crate::core::{GameConfig, GameState, Turn, Spell};
use crate::general::Node;

use anyhow::{bail, Context};

use std::str::FromStr;
use std::time::Instant;

/// Options for configuring the search behavior
#[derive(Debug, Clone)]
pub struct SearchOptions {
    /// Whether to search indefinitely
    // pub infinite: bool,
    /// Maximum time to search in milliseconds
    pub move_time: u64,
    /// Maximum number of nodes to search
    pub nodes: u64,
    // spells drawn for the search
    pub spells: Vec<Spell>,
}

impl FromStr for SearchOptions {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut i = 1;
        let mut search_options = SearchOptions::default();

        let parts = s.split_whitespace().collect::<Vec<_>>();

        while i < parts.len() {
            match parts[i] {
                "movetime" if i + 1 < parts.len() => {
                    let time = parts[i + 1].parse().context("invalid movetime")?;
                    search_options.move_time = time;
                    i += 1;
                }
                "nodes" if i + 1 < parts.len() => {
                    let n = parts[i + 1].parse().context("invalid nodes")?;
                    search_options.nodes = n;
                    i += 1;
                }
                "spells" if i + 1 < parts.len() => {
                    search_options.spells = parts[i + 1].split(',')
                        .map(|s| s.parse().context("invalid spell"))
                        .collect::<Result<Vec<_>, _>>()?;
                    i += 1;
                }
                p => bail!("invalid go argument {}", p)
            }
            i += 1;
        }
        Ok(search_options)
    }
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            // infinite: false,
            move_time: 1000,
            nodes: u64::MAX,
            spells: Vec::new(),
        }
    }
}

pub fn search_no_spells<'g>(config: &'g GameConfig, state: &GameState, search_options: &SearchOptions) -> Node<'g> {
    let now = Instant::now();
    let mut node = Node::new(config, state.clone());

    for _ in 0..search_options.nodes {
        node.explore();

        if now.elapsed().as_millis() > search_options.move_time.into() {
            break;
        }
    }

    node
}

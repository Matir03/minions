# Spooky Documentation

This directory contains detailed documentation for the Spooky engine, a sophisticated game-playing engine that uses Monte Carlo Tree Search (MCTS) with multi-stage move generation.

## Contents

- [Architecture Overview](architecture.md) - High-level system design and components
- [Core Representation](representation.md) - Game state and action data structures
- [Search System](search.md) - MCTS implementation and static evaluation
- [Move Generation](move_generation.md)
  - [General Stage](stages/general.md) - Tech decisions
  - [Attack Stage](stages/attack.md) - Combat resolution system
  - [Blotto Stage](stages/blotto.md) - Resource allocation
  - [Spawn Stage](stages/spawn.md) - Unit placement

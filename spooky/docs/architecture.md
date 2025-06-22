# Architecture Overview

The Spooky engine is built around a sophisticated Monte Carlo Tree Search (MCTS) implementation. The core MCTS nodes are `GameNode`s, which coordinate decisions from a `GeneralNode` (for game-wide strategy like technology choices) and multiple `BoardNode`s (one for each game board, handling board-specific actions like attacks and unit spawns). This document provides a high-level overview of the system architecture.

## Core Components

### Search System
- MCTS-based search with static evaluation
- Confidence interval-based pruning for exploration optimization
- Neural network evaluation (NNUE) for position assessment
- Backpropagation of feedback through all stages

### Move Generation and Decision Structure
Move generation within a `GameNode` involves a coordinated process:

1.  **`GameNode` Expansion**:
    *   When a `GameNode` is selected for expansion in the MCTS, it prepares to create child `GameNode`s representing subsequent game states.

2.  **`GeneralNode` Decisions**:
    *   The `GameNode` utilizes its associated `GeneralNode`.
    *   The `GeneralNode` makes high-level strategic decisions, primarily focusing on technology research and other game-wide parameters.

3.  **Money Distribution (Blotto Logic)**:
    *   The `GameNode` calls a dedicated function (e.g., `distribute_money`) to allocate the current player's available money.
    *   This money is distributed among the `GeneralNode` (for tech costs) and the various `BoardNode`s (for unit actions on each board).

4.  **`BoardNode` Decisions**:
    *   Each `BoardNode` (one per game board) receives its share of allocated money and the tech decisions from the `GeneralNode`.
    *   It then performs its own internal search or decision-making process to determine the best set of actions for its board. This includes:
        *   **Combat Actions**: Managing unit movements, attacks, and resolving engagements. The Z3 constraint solver is used to determine optimal attacks and movements.
        *   **Unit Spawning**: Deciding which units to create and where to place them using a heuristic-based approach.

5.  **Child `GameNode` Creation**:
    *   The collective decisions and outcomes (delta money, delta points) from the `GeneralNode` and all `BoardNode`s are aggregated.
    *   This information is used to create new child `GameNode`s with updated game states.

## Key Features

### Lazy Refinement
- `BoardNode`s may implement lazy generation of refined candidate moves (e.g., for attacks or spawns) internally.
- Coarse-to-fine refinement can occur within a `BoardNode`'s decision process based on MCTS feedback propagated to the parent `GameNode`.
- Efficient pruning of unpromising branches is a general MCTS feature applied to `GameNode` selection.

### Neural Network Integration
- Static evaluation with confidence intervals
- Initial move candidate generation
- Position evaluation using NNUE
- Efficient incremental updates for small position changes

### Constraint Satisfaction
- Robust constraint graph system for combat and repositioning resolution
- Efficient SAT solver integration for determining combat and repositioning actions
- Handling of complex game rules and interactions

## Implementation Notes

- `GameNode` manages the overall game state.
- `GeneralNode` and `BoardNode`s manage their respective sub-problems and their internal states.
- `GameNode` coordinates the decisions from `GeneralNode` and `BoardNode`s. `BoardNode`s operate largely independently of each other, given a set of general tech decisions and money allocation.
- Efficient caching and incremental updates remain important design goals.

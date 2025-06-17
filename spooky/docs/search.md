# Search System

The Spooky engine uses Monte Carlo Tree Search (MCTS) as its primary search algorithm, enhanced with several sophisticated components for improved performance.

## MCTS Implementation

### Core Components
- The MCTS tree is composed of `GameNode`s, each representing a game state and tracking potential future moves and outcomes.
- UCT-style selection (or a variant) is used to choose `GameNode`s for expansion, potentially with confidence interval modifications.
- Backpropagation of evaluation scores (e.g., from an `Eval` struct) and other metrics occurs up the `GameNode` tree.
- `GameNode` expansion is a coordinated process involving its internal components: a `GeneralNode` and multiple `BoardNode`s.

### Static Evaluator
- Neural network-based position evaluation
- Confidence interval generation for move quality
- Pruning of low-confidence or poor-quality moves
- Integration with NNUE for efficient updates

## `GameNode` Expansion Process

When a `GameNode` is selected for expansion:

1.  **Money Distribution (Blotto Logic)**:
    *   The `GameNode` first determines how to allocate the current player's available money. This is handled by a dedicated function (e.g., `distribute_money`).
    *   Resources are assigned to the `GeneralNode` (for technology costs) and to each of the `BoardNode`s (for actions on their respective boards).

2.  **`GeneralNode` Decisions**:
    *   The `GameNode`'s associated `GeneralNode` uses its allocated money to make high-level strategic decisions, such as researching technologies. These decisions influence the capabilities and options available to `BoardNode`s.

3.  **`BoardNode` Action Generation**:
    *   Each `BoardNode` (one for every board in the game) receives its share of money and the current technology state (from the `GeneralNode`).
    *   It then performs its own internal decision-making process (which could be a smaller, localized search or a heuristic procedure) to determine the best set of actions for its board. This includes:
        *   **Combat Actions**: Managing unit movements, attacks, and resolving engagements (formerly the Attack Stage).
        *   **Unit Spawning**: Deciding which units to create and where to place them (formerly the Spawn Stage).

4.  **Child `GameNode` Creation**:
    *   The outcomes from the `GeneralNode` (e.g., new tech state) and all `BoardNode`s (e.g., board configurations, delta points, delta money spent) are collected.
    *   This combined information is used to form one or more new child `GameNode`s, representing the resulting game states after these decisions.

### Component Dependencies within `GameNode`

-   The `GeneralNode`'s decisions (especially technology) provide context for `BoardNode` operations.
-   `BoardNode`s operate largely independently of each other once they receive their money allocation and the general tech state.
-   The overall `GameNode` coordinates these components to produce coherent full-game turns.

### Feedback Integration

-   Evaluation of a new child `GameNode` (e.g., using `Eval::static_eval`) provides a score.
-   This score is backpropagated up the MCTS tree: from the child `GameNode` to its parent, and so on.
-   Internally, the `GeneralNode` and `BoardNode`s also update their own statistics based on the outcomes of their decisions, contributing to the overall evaluation of the `GameNode`.

## Move Refinement

### Candidate Generation
- Candidate actions are generated within the `GeneralNode` (for tech choices) and `BoardNode`s (for board-specific actions like moves, attacks, spawns).
- These components might use lazy generation or progressive refinement internally, guided by heuristics, local search, or neural network outputs.
- Constraint satisfaction may be employed within `BoardNode`s, for example, to ensure valid combat sequences or unit placements.

### NNUE Integration
- Efficient incremental position evaluation
- Context-aware scoring considering global position
- Fast updates for small board changes
- Confidence metrics for evaluation reliability

## Performance Optimizations

### Pruning Strategies
- Confidence interval-based exploration pruning
- Early termination of unpromising branches
- Efficient memory management for large trees

### Parallel Processing
- Independent stage processing where possible
- Efficient state sharing between parallel processes
- Lock-free data structures for high throughput

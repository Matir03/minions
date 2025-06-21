# Combat Stage Logic

This document details the constraint satisfaction system used for the `CombatStage` of the AI's move generation pipeline. This stage is responsible for determining optimal combat actions and movements for all friendly units on a given board.

## Overview

The `CombatStage` uses the Z3 constraint solver to model the complex decision-making process of a combat phase. It builds a `CombatGraph` representing all possible interactions and then applies a set of constraints to find the best possible set of actions.

A key design evolution in this stage was the **removal of the `passive` variable**. The previous model used a boolean variable (`p_x`) to indicate whether a unit was passive (not attacking). This led to a critical flaw: if no profitable attacks were available, the solver would be forced to make all units passive, which prevented them from moving at all. This resulted in the solver finding no valid solutions.

The current model decouples movement from attacking, leading to a more robust and flexible system.

## Constraint Model

The new model enforces the following core principles:

1.  **All Units Must Move**: Every friendly unit is required to move to a valid, unoccupied hex within its movement range. This is a fundamental constraint that ensures the solver can always produce a valid set of moves, even if no attacks occur.

2.  **Attacks are Optional**: A unit *may* attack an enemy unit if it is in range from its destination hex. The solver is incentivized to make profitable attacks, but it is not forced to.

3.  **Hex Occupancy**: The solver ensures that no two units move to the same hex.

### Key Variables

-   **`unit_at_hex_{unit_id}` (Int)**: Represents the destination hex for a given friendly unit.
-   **`attack_{attacker_id}_{target_id}` (Bool)**: A boolean indicating whether an attack occurs between two units.

### Objective Function

The solver's objective is to maximize a combination of factors, including:
-   Damage dealt to enemy units.
-   Minimizing damage received.
-   Strategic positioning (handled by the subsequent `RepositioningStage`).

## Interaction with Repositioning Stage

After the `CombatStage` determines the optimal set of attacks and the corresponding movements for the attacking units, the `RepositioningStage` takes over. It uses the same Z3 solver instance to determine the final positions for all the non-attacking friendly units, typically moving them towards a strategically advantageous position like the enemy's center of mass.
2. Use NNUE for position evaluation
3. Generate refinements:
   - Fine: Modify existing solver state
   - Coarse: Add noise to prophecy probabilities

## Evaluation

### NNUE Integration
- Evaluate post-attack positions
- Consider global position context
- Efficient incremental updates
- Track evaluation confidence

### Refinement Selection
- Use MCTS values and confidence
- Balance exploration/exploitation
- Track refinement success rates

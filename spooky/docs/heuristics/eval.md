# Evaluation Heuristics

This document outlines the high-level evaluation function. For a more detailed breakdown of the complete AI decision-making process, see [strategy.md](./strategy.md).

## Evaluation Function

The evaluation is based on the total "dollar difference" between the two players, which is then converted to a final score using a scaled sigmoid function.

### 1. Compute Total Dollar Difference

The dollar difference is the sum of the following components:

*   **Dollars on Boards**: The net value of all units currently on all boards.
*   **Dollars on Tech Line**: The net value of all unlocked and acquired technologies.
*   **Board Points Value**: The current board points converted to a dollar value at a constant rate (e.g., 30 dollars per point).

### 2. Convert to Final Score

The total dollar difference is passed through a scaled sigmoid function to produce the final evaluation score, which typically ranges from -1 to 1.
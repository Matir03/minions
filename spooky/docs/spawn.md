# Heuristic-Based Spawn Logic

This document details the heuristic-based system for purchasing and spawning new units. This logic is executed after the Combat and Repositioning stages, using the money remaining after any combat actions.

Originally, this stage used the Z3 constraint solver to find optimal spawn locations. However, this approach proved to be a significant performance bottleneck, especially when a large amount of money was available to purchase multiple units. To resolve this, the spawn stage was refactored to be purely heuristic-driven, removing the dependency on Z3 entirely.

## Overview

The spawning process is broken down into two main heuristic-driven steps:

1.  **Unit Purchasing**: Decides which units to buy based on available money and technology.
2.  **Unit Placement**: Places the purchased units onto valid, empty hexes in the spawn zone.

This logic is implemented in the `generate_heuristic_spawn_actions` function.

### 1. Purchase Heuristic

The purchasing logic follows a simple, greedy algorithm:

-   A list of all units that can be purchased (based on the current tech unlocks) is created.
-   This list is sorted by unit cost in ascending order.
-   The AI repeatedly buys the cheapest available unit until it can no longer afford it.

This ensures that the AI spends its money as efficiently as possible to maximize the number of units on the board.

### 2. Placement Heuristic

Once the units to be spawned are determined, they are placed on the board according to the following heuristic:

-   The list of purchased units is sorted by cost in *descending* order. This prioritizes placing the most valuable and expensive units first.
-   A list of all valid, empty spawn locations is generated.
-   To provide a consistent and strategically sound placement order, the list of available spawn locations is sorted by their proximity to the center of the board.
-   The AI iterates through the sorted list of units and places each one in the next available spawn location.

This approach ensures that the most powerful units get the most central and presumably advantageous positions in the spawn zone.

# Spawn Placement Logic for BoardNode

This document details a constraint satisfaction system intended for optimizing unit placement (both for newly spawned units and existing units that need repositioning). This logic is designed to be encapsulated within `BoardNode`s, which manage actions on individual game boards as part of the overall `GameNode` MCTS expansion. Decisions here would use the money allocated to the board by the blotto distribution.

## Overview

This approach uses a constraint satisfaction system to determine the optimal placement of units on the board. It can be used for positioning newly spawned units (after purchase) and for repositioning other friendly units that did not engage in attacks. The goal is to optimize unit placement based on the current board state and available units.

For each piece, use a heuristic to compute the value of the piece at every hex it can legally move to. Then, use a constraint solver to find the best placement for each piece.

## Variables

For each piece `x` that did not attack (possibly floating because it was displaced) and hex `s` it can move to:
- $m_{xs}$ (bool): whether `x` moves to `s`
- $v_x$ (int): value of `x` after moving

## Constraints

1. Each piece `x` must move somewhere:
    - $$exactlyOne_s(m_{xs})$$
2. Each hex `s` can have at most one piece:
    - $$atMostOne_x(m_{xs})$$
3. Value is computed based on the hex moved to:
    - $$v_x = \sum m_{xs} * value(x, s)$$

## Objective Function
$$\sum_x v_x$$

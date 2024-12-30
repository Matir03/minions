# Spawn Stage

The spawn stage handles unit placement decisions, operating with dependencies on all previous stages. This stage optimizes the positioning of new units based on the current game state and planned actions.

## Overview

Use a constraint satisfaction system to determine the optimal placement of units on the board. This stage optimizes the positioning of new units based on the current game state and planned actions.

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

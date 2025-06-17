# Attack Resolution Logic for BoardNode

This document details a sophisticated constraint satisfaction system intended for handling combat resolution. This logic is designed to be encapsulated within `BoardNode`s, which manage actions on individual game boards as part of the overall `GameNode` MCTS expansion.

## Overview

Combat resolution within a `BoardNode` can be processed in several phases:
1. Combat Pair identification
2. Constraint graph construction for attack planning
3. Candidate move generation based on solving constraints
4. Evaluation of resulting board states

## Combat Pair Identification

For each friendly piece:
1. Compute all potentially targetable enemy pieces
2. Consider all possible attack hexes, including those blocked by enemy pieces
3. Generate all valid attacker-defender pairs
4. Map all possible attack hexes for each pair

## Constraint Graph

### Variables

#### Defender Variables
For each defending piece `y`:
- `r_y` (bool): Whether piece is removed
- `tr_y` (int): Time of removal
- `k_y` (bool): Whether piece is killed
- `u_y` (bool): Whether piece is unsummoned

#### Attacker Variables
For each attacking piece `x`:
- `p_x` (bool): Whether piece is passive (no combat)
- `ta_x` (int): Combat engagement time
- `m_xs` (bool): For each hex `s`, whether `x` moved to `s`

#### Combat Variables
For each attacker/defender pair `x,y`:
- `a_xy` (bool): Whether `x` attacked `y`
- `d_xy` (int): Effective damage dealt by `x` to `y`

### Constraints

#### Hex Constraints
For each relevant hex `s`:
- At most one piece can move to `s`: `atMostOne(m_xs)`
- Movement timing constraints:
  ```
  m_xs => f_x({ta_x > tr_y: y relevant in blocking m_xs})
  ta_x > ta_z if z is an attacking piece starting on s
  ```

#### Attacker Constraints
For each attack-relevant piece `x`:
1. Movement:
   - `exactlyOne(p_x, m_xs)`
2. Range:
   - `a_xy => exactlyOne(m_xs, s within range of y)`
3. Timing:
   - `a_xy => ta_x <= tr_y`
4. Attack limits:
   - `num(a_xy) <= maxAttacks(x)`
5. Damage calculation:
   - Flurry: `sum(d_xy) <= maxDamage(x)`
   - Star attack on persistent: `d_xy = a_xy`
   - Deathtouch on necromancer: `d_xy = 0`
   - Otherwise: `d_xy = a_xy * attack(x)`
     - Where `attack(x) = inf` for unsummon/deathtouch

#### Defender Constraints
For each defending piece `y`:
1. Removal conditions:
   - `exactlyOne(~r_y, k_y, u_y)`
2. Unsummon handling:
   - `u_y = exactlyOne(a_xy where x has * attack)`
3. Damage resolution:
   - Let `d_y = sum(d_xy)`
   - `r_y = (d_y >= defense(y))`
   - Optional no-overkill: `exists x, d_y - d_xy < defense(y)`

## Move Generation

### Prophecy System
1. Use neural network to generate 3-6 "prophecies"
2. Each prophecy maps pieces to probabilities of:
   - Attacking pieces being passive
   - Defending pieces being removed

### Candidate Move Generation
1. Sort pieces by probability (threshold ≈ 0.5)
2. Initialize constraint solver
3. Add constraints iteratively:
   - Add `p_x` or `r_y` constraints in probability order
   - Backtrack on UNSAT
4. Generate and verify move from SAT model
5. Verify piece evacuation feasibility

### Move Refinement
1. Track generated moves with constraints
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

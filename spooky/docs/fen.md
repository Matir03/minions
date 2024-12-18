# Spooky FEN Format

The Spooky FEN (Forsyth-Edwards Notation) format is a string representation that completely describes a game state. The format consists of several sections separated by spaces:

## Format

```
<num_boards> <map_indices> <num_techs> <tech_indices> <board_states> <side_to_move> <tech_status> <money>
```

### Static Configuration Section
1. **Number of Boards** - Single integer
   Example: `2`

2. **Map Indices** - Comma-separated list of map indices, no spaces
   Example: `0,1`

3. **Number of Techs** - Single integer
   Example: `4`

4. **Tech Indices** - Comma-separated list of tech indices defining the permutation, no spaces
   Example: `0,1,2,3`

### Game State Section
5. **Board States** - Each 10x10 board's state separated by "|"
   - Empty squares: number of consecutive empty squares (note: '0' represents 10 empty squares)
   - Rows separated by "/"
   - Pieces are represented by a single character:
     - Uppercase letters for Side 0 (e.g., 'Z' for Zombie)
     - Lowercase letters for Side 1 (e.g., 'z' for Zombie)
   Example: `N8z/0/0/0/0/0/0/0/0/0|0/0/0/0/0/0/0/0/0/0`

6. **Side to Move** - "0" or "1"

7. **Tech Status** - Two groups separated by "|", one for each side
   - Each tech: "L" (Locked), "U" (Unlocked), or "A" (Active)
   Example: `LLLUUA|LLLLLA`

8. **Money** - Two numbers separated by "|" for each side's money
   Example: `10|5`

## Unit Labels
- Z/z: Zombie
- I/i: Initiate
- S/s: Skeleton
- R/r: Serpent
- W/w: Warg
- G/g: Ghost
- T/t: Wight
- H/h: Haunt
- K/k: Shrieker
- P/p: Spectre
- A/a: Rat
- C/c: Sorcerer
- V/v: Vampire
- M/m: Mummy
- L/l: Lich
- O/o: Void
- B/b: Banshee
- E/e: Elemental
- Y/y: Harpy
- D/d: Shadowlord
- N/n: Necromancer

## Examples

1. Initial position, 2 boards, maps 0 and 1, 4 techs in default order:
   ```
   2 0,1 4 0,1,2,3 N8z/0/0/0/0/0/0/0/0/0|0/0/0/0/0/0/0/0/0/0 0 LLLUUA|LLLLLA 10|5
   ```

2. Mid-game position with multiple units:
   ```
   2 0,1 4 0,1,2,3 Z3n2z3/0/2S5s1/0/0/0/G7g1/0/0/0|0/0/V7v1/0/0/0/0/0/0/0 1 LUUAAL|LLLAAU 20|15
   ```

## Parsing Rules

1. All sections must be present in the specified order
2. Board positions are read from top-left to bottom-right for each 10x10 board
3. Map and tech indices must be valid for the game configuration
4. Number of techs must match the length of tech indices list and tech status strings
5. The digit '0' represents exactly 10 empty squares in board representation
6. Each row must sum to 10 squares (counting units as 1 and numbers as their value, except '0' which counts as 10)

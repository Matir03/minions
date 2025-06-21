# Minions Game Rules for AI

## 1. Game Objective

The primary goal is to be the first team (Yellow or Blue) to achieve a target number of "board points". Board points are earned by winning individual boards.

## 2. Core Components

- **Boards:** 10x10 hexagonal grids where combat occurs. Boards contain special "graveyard" tiles that generate income.
- **Techline:** A linear progression of unit types. Players unlock new units by investing in the techline.
- **Units:** Game pieces on the boards. Each has stats (Attack, Defense, Speed, Range) and may have special keywords (e.g., Flying, Spawn).
- **Necromancer:** A special, critical unit on each board. If a team's Necromancer is destroyed, that team loses the board.
- **Money:** A shared resource for each team, used to purchase units and spells for the techline.

## 3. Gameplay Overview

- The game is turn-based, with Yellow moving first.
- Each team has two primary roles: a **General** (manages the techline) and **Captains** (manage units on each board).

### Turn Phases

#### General's Phase (Techline)

1.  **Acquire Spells:** Receive one free spell per turn and purchase more with money.
2.  **Tech Up:** Place spells on tech cards to unlock new unit types for the entire team. Unlocking a unit type requires two spells on its card and blocks the opponent from teching to it.

#### Captain's Phase (Boards)

1.  **Attack Phase:**
    - **Move:** Move units up to their speed limit.
    - **Attack:** Use units to attack enemy units within range. Damage is tracked per turn.
2.  **Spawn Phase:**
    - **Purchase Units:** Buy new units from the team's unlocked tech tree.
    - **Spawn Units:** Place purchased units on the board adjacent to friendly units with the "Spawn" keyword.

## 4. Winning and Losing

### Winning/Losing a Board

A board is won/lost in one of the following ways:

- **Necromancer Kill (Win):** Destroy the enemy's Necromancer. The board resets, and you gain 1 board point.
- **Graveyard Control (Loss):** The opponent controls 8 or more graveyards at the end of their turn. The board resets, and your opponent gains 1 board point.
- **Resignation (Loss):** A team can choose to resign a board.

### Board Reset

When a board is won or lost, it resets:
- Most units are returned to a reinforcement zone.
- Each side chooses a new advanced Necromancer for that board.
- Play resumes on the reset board, with a brief period of adjusted turn phases.

### Winning the Game

The first team to accumulate the required number of board points (typically `n - floor(n/4)` where `n` is the number of boards) wins the game.

## 5. Economy

- **Income:** At the end of a turn, each team earns money based on the number of graveyards they control across all boards.
- **Spending:** Money is spent on purchasing new units for the boards and additional spells for the techline.

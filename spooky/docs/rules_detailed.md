# Minions: Detailed Rules Specification

## 1. Game Objective

Win the game by being the first side (Yellow or Blue) to accumulate `w(n)` board points, where `n` is the number of boards and `w(n) = n - floor(n/4)`.

## 2. Game Components

### 2.1. Boards
- **Layout:** 10x10 hexagonal grid in a rhombus shape. Coordinates are `a-j` and `0-9`.
- **Tiles:**
  - **Land:** Standard terrain.
  - **Water:** Passable only by units with the `Flying` keyword.
  - **Graveyard:** Generates income. Controlled if occupied by a friendly unit at the end of your turn.
- **Invariants:** Boards are symmetric (rotationally or reflectively), have exactly 10 non-adjacent graveyards, and guarantee land tiles at `c2`, `h7`, and their adjacent hexes.

### 2.2. Techline
- A sequence of `m` tech cards, each corresponding to a unit type.
- Unit `i` is designed to counter units `i-1`, `i-2`, and `i+3`.
- Unlocking a unit requires placing two spells on its tech card.

### 2.3. Units
- **Stats:**
  - **Attack:** Damage output and type.
    - `Normal`: One attack per turn, fixed damage.
    - `Flurry`: Multiple 1-damage attacks, up to a total fixed damage.
    - `Unsummon`: Multiple attacks. Sends normal units to reinforcements; deals 1 damage to `Persistent` units.
    - `Deathtouch`: One attack per turn. Instantly destroys any non-Necromancer unit.
  - **Defense:** Health. Damage resets at the end of each turn.
  - **Speed:** Movement range in hexes.
  - **Range:** Attack distance in hexes.
  - **Cost:** Price to purchase.
  - **Rebate:** Money refunded upon the unit's death.
- **Keywords (Abilities):**
  - `Flying`: Can move over water and enemy units.
  - `Lumbering`: Cannot move and attack in the same turn.
  - `Spawn`: Can spawn new units in adjacent hexes.
  - `Persistent`: Immune to `Unsummon` (takes 1 damage instead).
  - `Necromancer`: If this unit dies, its side loses the board. Can have unique keywords:
    - `soul`: Board generates +1 income per turn.

### 2.4. Spells
- Used exclusively to tech to new units. One free spell is gained per turn; more can be bought.

## 3. Game Setup

1.  **Parameters:** Define number of boards (`n`), win condition (`w(n)`), specific boards, techline, and starting money.
2.  **Techline Setup:** The first `k` cards are fixed. The rest are split into two packs, sorted, and interleaved to form the remaining techline.
3.  **Board Setup:**
    - Yellow starts with a Necromancer on `c2` and 6 Zombies on adjacent hexes.
    - Blue starts with a Necromancer on `h7` and 6 Zombies on adjacent hexes.
    - Both sides get an Initiate in their reinforcement zone.
4.  **Starting Money (Blue):** Determined by a fixed amount (e.g., `6n`) or a bidding process.

## 4. Turn Structure & Phases

Turns are taken alternately (Yellow first) and proceed in parallel across the techline and all boards.

### 4.1. General (Techline)
- **Actions:**
  - Receive 1 free spell. Purchase more for `$8n` each.
  - **March:** Place a spell on the first un-spelled tech card.
  - **Tech:** Place a second spell on a card where you have one spell and the opponent has at most one. This unlocks the unit and blocks the opponent from teching to it.

### 4.2. Captain (Boards)
Board state determines available phases:

- **Normal Turn:** Attack Phase -> Spawn Phase.
- **Post-Reset Turns:** Boards follow a specific sequence of states (`reset+0`, `reset+1`, `reset+2`) with restricted phases to manage re-entry into the game.

#### **Phases**

1.  **Choose Necromancer Phase (After a Board Reset):**
    - Select a new, unused advanced Necromancer.
    - Keep one unit from the reinforcement zone; discard the rest.
    - Gain a new Initiate for free.

2.  **Attack Phase:**
    - Actions occur in simultaneous 'ticks'.
    - **Movement:** A unit can move once per turn if it hasn't attacked. It requires a clear path (no enemy units, unless `Flying`). Multiple friendly units can swap positions in the same tick.
    - **Attacking:** A unit can attack if it hasn't moved (unless it has a special keyword). Attacks are subject to range and type restrictions. Units are destroyed at the end of a tick if damage meets or exceeds defense.

3.  **Spawn Phase:**
    - **Purchase:** Buy any number of unlocked units.
    - **Spawn:** Place purchased units from the reinforcement zone onto empty hexes adjacent to a friendly `Spawn` unit.

## 5. End of Turn Sequence

Events are resolved in this strict order:

1.  **Income:** Collect money from controlled graveyards (`g`), plus any `soul` bonus (`s`). Total board income = `g + s + 2`.
2.  **Board Wins (Necromancer Kill):** If you destroyed an enemy Necromancer, you win that board and gain 1 board point.
3.  **Game Win Check:** If your new total board points meet or exceed `w(n)`, you win the game.
4.  **Board Losses (Graveyard Control / Resign):** You lose any board where the opponent controls >= 8 graveyards or that you resign. Your opponent gains 1 board point.
5.  **Game Win Check (Opponent):** If the opponent's new total meets or exceeds `w(n)`, they win the game.
6.  **State Change:** Board states are updated for the next turn.

## 6. Board Reset

When a board is won or lost, it resets. The process involves a specific sequence of states and actions to re-integrate the board into play.

### 6.1. Initial Reset State
- All units on the board are returned to their respective owner's reinforcement zone.
- Each side receives 6 Zombies on the hexes adjacent to their starting positions (`c2` for Yellow, `h7` for Blue).

### 6.2. Post-Reset Turn Sequence
The board's state transition depends on which side won:

- **If you won the board:** The board enters the **`reset + 0`** state for the opponent.
  - **Opponent's Turn (`reset + 0`):** The opponent must pass their turn (no phases). The board then transitions to the **`reset + 1`** state for you.
- **If you lost the board:** The board enters the **`reset + 1`** state for your opponent.

The sequence for the player whose turn it is proceeds as follows:

1.  **`reset + 1` Turn:**
    - **Phase 1: Choose Necromancer:**
      - Select a new, previously unused advanced Necromancer.
      - Choose one unit from your reinforcement zone to keep; all others are removed.
      - A new Initiate is added to your reinforcement zone for free.
    - **Phase 2: Attack Phase:** Normal attack phase.
    - **(No Spawn Phase)**
    - The board transitions to the **`reset + 2`** state for the other player.

2.  **`reset + 2` Turn:**
    - **Phase 1: Choose Necromancer:** The other player performs the same necromancer selection process.
    - **Phase 2: Attack Phase:** Normal attack phase.
    - **Phase 3: Spawn Phase:** Normal spawn phase.
    - The board transitions to the **`normal`** state for both players, resuming standard gameplay on the next turn.

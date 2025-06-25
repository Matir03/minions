# Core Representations

This document details the core data structures used to represent game state and actions in the Spooky engine.

## Game Structure

### Game
Composed of:
- `GameConfig`: Static game configuration
- `GameState`: Dynamic game state

### GameConfig
- `numBoards`: int
- `maps`: Vec<Arc<Map>>
- `techline`: Techline

### GameState
- `config`: Arc<GameConfig>
- `side_to_move`: Side
- `boards`: Vec<Board>
- `techStatus`: SideArray<TechStatus[numTechs]>
  - `TechStatus`: enum {Locked, Unlocked, Acquired}
- `money`: SideArray<int>

## Map Components

### Map
- `hexes`: HexArray<TileType>
- `TileType`: enum {Land, Water, Graveyard, ...}

## Tech System

### Techline
- `numTechs`: int
- `Tech`: int[numTechs]

### Tech
One of:
- `UnitTech(UnitLabel)`
  - `UnitLabel`: enum {Zombie, Initiate, ...}
  - Maps to global Vector<Unit>
- `Copycat`
- `Thaumaturgy`
- `Metamagic`

### Unit
- `attack`: Attack
  - Attack = Damage(int) | Unsummon | Deathtouch
- `numAttack`: int
- `defense`: int
- `speed`: int
- `range`: int
- `cost`: int
- `rebate`: int
- Flags:
  - `necromancer`: bool
  - `lumbering`: bool
  - `flying`: bool
  - `persistent`: bool
  - `spawn`: bool

## Board State

### Board
- `map`: Arc<Map>
- `state`: BoardState
- `pieces`: HashMap<Loc, Piece>
- `reinforcements`: SideArray<HashBag<Unit>>
- `spells`: SideArray<Vec<Spell>>
- `winner`: Option<Side>

### Piece
- `loc`: Loc
- `unit`: UnitLabel
- `modifiers`: Modifiers
  - Modifiers = {
    - `shielded`: bool
    - `frozen`: bool
    - `shackled`: bool
    - ...
  }

### Spell
One of:
- Shield
- Reposition
- ...

## Game Actions

### GameTurn
Represents a complete turn for one player.
- `board_turn`: BoardTurn
- `tech_assignment`: TechAssignment
- `spell_casts`: Vec<SpellCast>

### BoardTurn
A collection of actions for a single board, organized by phase.
- `setup`: Vec<SetupAction>
- `attack`: Vec<AttackAction>
- `spawn`: Vec<SpawnAction>

### SetupAction
Actions for setting up the board state.
- `Add { piece, loc }`: Add a piece to the board.
- `Remove { loc }`: Remove a piece from the board.
- `Reset`: Reset the board to its initial state.

### AttackAction
Actions performed during the attack phase.
- `Move { from, to }`: Move a unit.
- `Attack { attacker_loc, target_loc }`: An attack by one unit on another.

### SpawnAction
Actions performed during the spawn phase.
- `Buy { unit }`: Purchase a unit.
- `Spawn { unit, spawn_loc }`: Spawn a unit at a specific location.
  - `from_sq`: Loc
  - `to_sq`: Loc
- Spawn
  - `spawn_sq`: Loc
  - `unit`: UnitLabel
- Cast
  - `spell`: Spell
  - `params`: CastParams
- Discard
  - `spell`: Spell
- EndPhase

# Core Representations

This document details the core data structures used to represent game state and actions in the Spooky engine.

## Game Structure

### Game
Composed of:
- `GameConfig`: Static game configuration
- `GameState`: Dynamic game state

### GameConfig
- `numBoards`: int
- `maps`: MapLabel[numBoards]
  - `MapLabel`: enum {BlackenedShores, MidnightLake, ...}
  - Maps to global Vector<Map>
- `techline`: Techline

### GameState
- `side_to_move`: Side
- `boards`: Board[numBoards]
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
- `pieces`: SideArray<Vector<Piece>>
- `reinforcements`: SideArray<Vector<UnitLabel>>
- `spells`: SideArray<Vector<Spell>>

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
- `num_spells_bought`: int
- `board_spells`: int[num_boards]  // board -> spell index
- `tech_spells`: Vector<int>  // techline indices
- `board_actions`: Vector<BoardAction>[num_boards]

### BoardAction
One of:
- Move
  - `from_sq`: Loc
  - `to_sq`: Loc
- Attack
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

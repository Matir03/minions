# Rules of Minions

## Summary
Minions is a strategy game played between two sides, Yellow and Blue, taking alternate turns. Each side can be abstractly thought of as a single player, though in practice a side can be made up of a team with multiple players. The game is played on a number of boards as well as a techline. The boards are 10x10 hexagonal grids laid out in the shape of a rhombus. Each side has a number of units on the board which fight each other for control of the board. The boards have special tiles called "graveyards" which generate income for the side that controls them at the end of their turn. The money generated from the boards is pooled together and is used to purchase units for the boards and extra spells for the techline. The techline consists of a linear sequence of tech cards for unit types. Each side puts spells under the tech cards in sequence, and can tech to a unit type by having two spells under the corresponding tech card. That side is then able to purchase that units of that type for the boards. Each side has a special unit called a necromancer on each board. A side can win a board by controlling enough graveyards or by killing the enemy necromancer. When a side wins a board, they gain a board point, and the board immediately resets, with play resuming on the board from the next turn. To win the game, a side must reach a certain number of board points.

## Game Components
The game is played on the following components.

### Board
A board is a 10x10 hexagonal grid laid out in the shape of a rhombus. Each side has a number of units on the board which fight each other for control of the board. The boards have special tiles called "graveyards" which generate income for the side that controls them at the end of their turn.

### Techline
A techline consists of a sequence of tech cards, numbered from `1` to `m`. The tech cards have the property that unit `i` counters units `i-1`, `i-2`, and `i+3` (if they exist). The tech cards each correspond to a unit type, and a side can tech to a unit type by having two spells under the corresponding tech card. That side is then able to purchase that units of that type for the boards.

### Unit
A unit is a game piece that can be placed on a board. Each unit has a type, which determines its stats. 

#### Unit Stats
Each unit has the following stats:
- Attack: Units have different types of attack, which determines how they can kill or remove enemy units. 
    - Normal: The unit can perform a single attack on any enemy unit within its range, dealing a fixed amount of damage.
    - Flurry: The unit can perform any number of 1 damage attacks on enemy units within its range, with total damage dealt up to a fixed amount.
    - Unsummon: The unit can perform up to a fixed number of unsummon attacks on enemy units within its range. An unsummon attack sends normal units to their owner's reinforcement zone, and deals 1 damage to persistent units.
    - Deathtouch: The unit can perform a single attack on any enemy unit within its range, excluding the enemy necromancer, instantly destroying the attacked unit.         
- Defense: The amount of damage the unit can take before being destroyed. Damage taken is reset at the end of the turn; only damage taken in the current turn counts.
- Speed: The number of hexes the unit can move in a turn.
- Range: The number of hexes the unit can attack from.
- Cost: The amount of money required to purchase the unit.
- Rebate: The amount of money the side with the unit gets back when the unit is destroyed.
- Keywords: These are special rules that modify the unit's behavior.

#### Unit Keywords
- Flying: The unit can move through water tiles or enemy units.
- Lumbering: The unit cannot both move and attack in the same turn.
- Spawn: The unit can spawn other units next to it.
- Persistent: The unit cannot be unsummoned. When it is attacked by an unsummon attack, it instead takes 1 damage.
- Necromancer: This is a special keyword only present on the necromancer. If this unit is destroyed, the board is reset to its initial state, and the side that destroyed it gets a board point. Each board starts with a basic necromancer, and whenever a board resets, each captain picks an advanced necromancer to start the board. There are many types of necromancers, each with different stats and keywords. There are some keywords that are only present on necromancers:
    - soul: the board generates an extra income of 1 per turn.

### Spell
A spell is a game piece that can be placed on a tech card. When playing with spells enabled, they can also be played by captains on their boards, where they have special effects depending on the spell; however, we will only describe playing with spells disabled, so we will not describe the types or effects of spells. In this version of the game, spells are only used to tech to units, and their types are irrelevant.

### Roles
There are two roles in the game: general and captain. The techline has a general playing on it, who is responsible for teching to units and purchasing spells. Each board has a captain playing on it, who is responsible for moving and attacking with their units, purchasing units, and spawning new units.

### Money
Each side has a pool of money shared by the general and captains. The amount of money each side has must be a non-negative integer at all times. A player cannot spend money to purchase a unit or spell if they do not have enough money.

## Game Setup
To start a game, the players must select the number of boards, the number of board points needed to win (which is usually a function of the number of boards, described below), the specific boards to use, the choice of techline, the layout of the tech cards (which is usually randomized as described below), whether to use spells (we will not use spells in this version of the game), which side each teams plays, and the amount of starting money blue gets as compensation for going second. 

For the rest of this document, let `n` be the number of boards.

### Techline Setup
To randomize the techline, for some fixed number `k` (depending on the techline) the first `k` cards are placed at the start of the techline in order, and the remaining cards are randomly split into two packs of equal size (if there's an odd number of cards, the second pack has one extra card), and each pack is sorted in ascending order. The two packs are then placed after the first `k` cards with the cards being chosen alternately from the two packs, with the first pack being used first.

### Board Setup
Boards are 10x10 grids of hexagonal tiles laid out in the shape of a rhombus. The files and ranks of the board are indexed from `a` to `j` and `0` to `9` respectively. Each tile on the board has a tile type, which is one of the following: 
- Graveyard: A tile that generates income for the side that controls it at the end of their turn.
- Water: A special tile that only allows flying units to stand or move through it
- Land: A normal tile that allows any unit to stand or move through it
A board consists of a fixed configuration of tiles. The configuration is usually selected from a set of pre-defined configurations, but can also be randomized. The following invariants are guaranteed for the configuration: 
- The configuration is guaranteed to be symmetric, either by reflection or rotation.
- The configuration is guaranteed to have exactly 10 graveyards, no two of which are adjacent.
- The hexes at `c2` and `h7`, as well as all of their neighbors, are guaranteed to be land.

#### Unit Setup
Each side starts with a basic necromancer, with Yellow's necromancer on `c2` and Blue's necromancer on `h7`, and a ring of `6` zombies of their color on the six hexes adjacent to their necromancer. Each side also starts with an initiate in their reinforcement zone.

### Number of Board Points
The number of board points needed to win `w(n)` is usually the following function of the number of boards `n`: 
`w(n) = n - floor(n/4)`

### Starting Money
There are two ways to decide the amount of starting money given to blue. 
- The first is to use a fixed amount, usually `6n`.
- In the second approach, the two teams place a bid for the amount of starting money they think blue should get for the game to be fair. The team that bids the lower number plays blue, while the team that bids the higher number plays yellow. 
The amount of starting money blue gets is the average of the two bids. The fractional part `f` of the average is rounded down with probability `1-f` and rounded up with probability `f`.

## Game Play
The game is played on alternate turns, with yellow going first. Each turn is played in parallel on each of the boards as well as the techline. Playing on the techline is called "generaling" while playing on the boards is called "captaining". The player who is generaling is called the "general" while the player who is captaining is called the "captain"; note that a player can simultaneously be playing as a captain on any number of boards and/or as a general on the techline. The amount of money the side has is shared between the general and all the captains; the players together cannot spend more than the amount of money they start the turn with.

### Generaling
The role of the general is to place spells under the tech cards to tech to units. Each turn, the general gets a spell for free, and can purchase additional spells at the cost of `$8n` per spell. The spells are placed under tech cards one by one. For each spell, the general can choose to place it under any of the following tech cards: 
- The first tech card that they have not placed a spell under. This is called "marching": the general is incrementing how far their side is along on the techline.
- Any tech card that they have exactly one spell under, and their opponent has at most one spell under (i.e. that their opponent hasn't teched to). This is called "teching". Their side can build a unit of the type corresponding to the tech card starting from the next turn. Furthermore, their opponent is blocked from teching to that tech card for the rest of the game.

### Captaining
How the turn proceeds on each board depends on the state of the board. There are several possible states for a board:
- The board is on the "turn 1" state. This is the state at the first turn of the game, and after a board is reset. On this turn, the player only has an attack phase, but no spawn phase. The board enters the "normal" state at the end of this turn.
- The board is in the "normal" state. This is the usual state of a board. On this turn, the player has an attack phase followed by a spawn phase. The board stays in the "normal" state at the end of this turn.
- The board is in the "reset + 0" state. This is the state of a board after the other side just won the board on the previous turn, and is going to go first on the next turn. On this turn, the player has no phases and must pass their turn, and the board enters the "reset + 1" state at the end of this turn.
- The board is in the "reset + 1" state. This is the state of a board after either the other side just lost the board on the previous turn or the current side won the board on the turn before and the other side was forced to pass the last turn. On this turn, the player has a "choose necromancer" phase, followed by an "attack phase", but no spawn phase. The board enters the "reset + 2" state at the end of this turn.
- The board is in the "reset + 2" state. This is the state of a board after the other side just took their first turn after winning their board. On this turn, the player has a "choose necromancer" phase, an "attack phase", and a "spawn phase", in that order. The board enters the "normal" state at the end of this turn.

Whenever the board is won, it enters the "reset + 0" state, and whenever it is lost, it enters the "reset + 1" state.

#### Choose Necromancer Phase
In the choose necromancer phase, the following three events happen:
- The captain chooses a new necromancer for the board. The necromancer must an advanced necromancer that their side has not yet chosen in this game. The necromancer is placed on its starting hex, which is `c2` for yellow and `h7` for blue. 
- Additionally, the captain chooses a single unit to keep in the reinforcement zone. All other units in the reinforcement zone are removed from the board. 
- An initiate is placed in the reinforcement zone for free.

#### Attack Phase
In the attack phase, the captain can move and attack with any of the units. A unit can move at most once per turn, and it cannot move if it has already attacked. 
The attack phase proceeds in any number of 'ticks', where a tick is a discrete unit of time. In a single tick, the captain can move any number of their units, or perform a single attacks, subject to restrictions. All the events in a tick happen simultaneously. The board is updated after all the events in a tick have occurred. 
The restrictions on movement are the following:
- A unit can only move to a hex that is either empty, or occupied by another friendly unit that is moving to a different hex in the same tick.
- There must be a path of hexes from the unit's current hex to the hex it is moving to, such that 
    - the length of the path is at most the unit's speed.
    - the path only contains hexes that the unit is allowed to move through. Units are allowed to move through a hex if one of the following is true:
        - The unit is flying.
        - The hex is land and does not contain an enemy unit at the start of the tick.
A unit can attack enemy units within its range, subject to the following restrictions based on its attack stats:
- Normal: The unit can perform at most one attack in the whole turn. The attack deals a fixed amount of damage.
- Flurry: The unit can perform any number of 1 damage attacks. The total number of attacks performed by the unit over the whole turn is at most the unit's damage.
- Unsummon: The unit can perform any number of unsummon attacks. The total number of attacks performed by the unit over the whole turn is at most the unit's damage. An unsummon attack unsummons normal units, sending them to their owner's reinforcement zone instantly, and deals 1 damage to persistent units.
- Deathtouch: The unit can perform a single attack on any enemy unit within its range, excluding the enemy necromancer, destroying the enemy unit instantly.
When the total damage taken by an enemy unit over the turn is equal to or greater than its health, it is instantly destroyed. "Instantly" here means that the unit is destroyed at the end of the tick, and the board is updated to reflect this. When a unit is destroyed, it is removed from the game, and the owner of the unit gets the unit's rebate.

#### Spawn Phase
In the spawn phase happens in two parts:
- Purchasing: The captain buys any number of units, paying the sum of their costs in money, subject to the following conditions:
    - All units must either be a basic unit, or a unit of a type that their side has teched to.
    - The captain cannot purchase more units than they have money for.
All purchased units are added to the captain's reinforcement zone.
- Spawn: The captain can simultaneously spawn any number of units from their reinforcement zone to any hex on the board with the following conditions:
    - The hex must be empty.
    - The hex must be adjacent to a friendly unit with the spawn keyword.
    - If the hex has water, the unit must have the flying keyword.

### End of Turn
Three things happen on a board at the end of a turn, in the following order:
- the side whose turn it is collects income from their boards
- the side whose turn it is wins the boards where they killed the enemy necromancer
- the side whose turn it is wins the game if they have won at least `w(n)` boards
- the side whose turn it is loses any boards where the opponent controls at least 8 gravyards or that they choose to resign
- the states of the boards change as appropriate and it becomes the other side's turn

##### Income
The amount of income generated by a board is `g+s+2`, where 
- `g` is the number of graveyard hexes containing a unit of the side's color
- `s = 1` if the side has a necromancer with the keyword "soul", and `s = 0` otherwise.
The total income generated by a side is the sum of the income generated by each of their boards. This income is added to their money.

##### Board Wins
The board is won by the current side if the opponent has no necromancer left on the board.
When a board is won, the board is reset and enters the "reset" state, and the side that just won it gets a board point. If the side has won at least `w(n)` boards after winning their boards, they win the game.

##### Board Losses
The board is lost by the current side if either 
- there are at least 8 enemy units on graveyards
- the current side resigns their board
When a board is lost, the board is reset and enters the "first turn" state, and the opponents gets a board point. If the opposing side has won at least `w(n)` board points after the board was lost, the opposing side wins the game.

##### Board Reset
When a board is reset, all units on the board are placed back into the reinforcement zone of the side that controls the unit, an additional initiate is placed in each reinforcement zone, and each side gets six zombies on their respective starting hexes, which are around `c2` and `h7`. 

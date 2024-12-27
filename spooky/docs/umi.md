# Universal Minions Interface

This is the documentation for the Universal Minions Interface protocol, based on the UCI protocol.

See [the Stockfish UCI documentation](https://official-stockfish.github.io/docs/stockfish-wiki/UCI-&-Commands.html) for more information about the UCI protocol.

## Commands

List of UMI commands and expected responses.

### `quit`
Quits the program.

### `umi`
Asks engine to use UMI.

#### Response
```
id name ___ author ___
option name __ type __ default __ [min __ max __]
...
umiok
```

### `setoption`
Sets an option for the engine.

#### Format
`setoption name __ value __`

#### List of options
| name | type | default | min | max |
|:---|:---|:---|:---|:---|
| spells | bool | false |  |  |
|:---|:---|:---|:---|:---|

### `isready`
Checks if the engine is ready.

#### Response
```
readyok
```

<!-- ### `newgame`
Indicates that the next positions are from a new game.

#### Format
`newgame <configstring>` -->

### `position`
Sets the position for the engine. Can only be called after `newgame`. Must be compatible with the current game config.

#### Format
`position (startpos | fen <fenstring>)`

### `go`
Perform a search on the current position.

#### Format
`go [movetime <time in ms>] [nodes <nodes>] [spells <spells>]` 
`spells` should be specified if spells are enabled

#### Response
Output current evaluation in the format `info eval <score>`.

### `play`
Play a turn from the current position.

#### Format
`play [movetime <time in ms>] [nodes <nodes>] [spells <spells>]`

#### Response 
The engine should respond with a sequence of actions representing its move, possibly along with search info. Every action should follow the format `action <actiontype> [actionparams...]`, and everyone information string should follow the format `info <infostring>`. Actions and info strings should be separated by newlines. Possible actions are

- `buyspell`
    -- the GUI should respond with a chosen spell in the format `spell <spellname>`
- `advancetech <numtechs>`
- `acquiretech <techindex>`
- `givespell <boardindex> <spellname>`
- `boardaction <boardindex> <boardaction> <boardactionparams...>`
    - `move <from> <to>`
    - `movecyclic <locs>`
    - `attack <from> <to>`
    - `blink <loc>`
    - `buy <unit>`
    - `spawn <loc> <unit>`
    - `cast <spellname> <params>`
    - `discard <spellname>`
    - `endphase`
    - `resign`
    - `saveunit <unit>`

After the last action, a final line `endturn` will signal the end of the turn.

### `turn`
Perform a turn on the current position.

#### Format
`turn [spells <spells>]`. Uses the same format as the `play` response for actions. Each `buyspell` follows the format `action buyspell <spellname>`.

### `display`
Output the current position of the game in ASCII format.

### `perft <board_index>`
Counts the number of distinct minimal attacking turns in the given position, as a debugging measure. 

An attacking turn is defined as a set of friendly pieces defined as "attacking", a map from each such friendly piece to their move, and two sets of enemy pieces identified as "killed" or "unsummoned" respectively, which satisfies the following property:
- assuming all non-attacking friendly pieces are removed from the board, there exists some sequence of moves and attacks by the attacking pieces such that the pieces perform their identified moves and the result of which is that the identified sets of enemy pieces are killed and unsummoned respectively.

An attacking turn is minimal if removing an attacking piece and its move from the turn results in a turn that is not an attacking turn.

Two attacking turns are distinct if the one of the underlying sets or the map of moves is distinct.

### `getfen`
Returns the current position in FEN format.
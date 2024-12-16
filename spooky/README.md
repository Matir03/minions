# Minions Board Game Engine

This is an implementation of the Minions board game, with a UCI-like interface for engine communication.

## Project Structure

- `minions-engine`: The core game logic library
- `minions-uci`: A binary implementing a UCI-like protocol for engine communication

## Building

```bash
cargo build
```

## Running

```bash
cargo run -p minions-uci
```

## UCI-like Protocol

The engine supports these commands:

- `uci`: Switch to UCI mode and identify the engine
- `isready`: Check if the engine is ready to accept commands
- `quit`: Exit the program

More commands will be added as the engine develops.

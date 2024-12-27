# Minions Board Game Engine

This is an implementation of the Minions board game, with a UMI interface for engine communication.

## Project Structure

- `minions-engine`: The core game logic library
- `minions-umi`: A binary implementing a UMI protocol for engine communication

## Building

```bash
cargo build
```

## Running

```bash
cargo run -p minions-umi
```

## UMI Protocol

See [docs/umi.md](docs/umi.md) for protocol documentation.

### Basic Commands

- `umi`: Switch to UMI mode and identify the engine
- `isready`: Check if the engine is ready to accept commands
- `quit`: Exit the program

More commands will be added as the engine develops.

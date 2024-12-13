# Minions Board Game

This repository contains the game engine and web-based GUI implementation for the Minions board game.

## Project Structure

- `/rust`: Contains the game engine implementation with UCI-like interface
  - `minions-engine`: Core game logic library
  - `minions-uci`: UCI-like protocol implementation

- `/typescript`: Web application
  - `/frontend`: React-based GUI
    - Built with Vite, React, and TypeScript
    - Provides a web interface to play the game
  - `/backend`: Express server
    - Manages communication between the GUI and the game engine
    - WebSocket-based real-time updates

## Development

### Game Engine (Rust)
```bash
cd rust
cargo build
```

### Backend (TypeScript)
```bash
cd typescript/backend
npm install
npm run dev
```

### Frontend (TypeScript)
```bash
cd typescript/frontend
npm install
npm run dev
```

## Architecture

The application is split into three main components:

1. **Game Engine** (Rust): Handles game logic and UCI-like protocol
2. **Backend** (TypeScript/Node.js): Manages communication between the frontend and the game engine
3. **Frontend** (React): Provides the user interface

Communication flow:
- Frontend ↔ Backend: WebSocket connection for real-time updates
- Backend ↔ Game Engine: Standard input/output through UCI-like protocol

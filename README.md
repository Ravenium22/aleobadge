# Match3 PVP - Real-time Match-3 Battle Game

A complete real-time multiplayer PvP match-3 game built with Rust. Features a game client (Macrosquad) and dedicated multiplayer server (Tokio + WebSockets).

## Project Structure

This is a Cargo workspace containing:

- **`client/`** - Game client with Macrosquad (desktop and mobile)
- **`server/`** - Multiplayer server with matchmaking and game synchronization
- **`protocol/`** - Shared protocol definitions for client-server communication

## Features

- **Real-time PvP**: Compete against an opponent to achieve the highest score
- **90-Second Matches**: Fast-paced gameplay with a countdown timer
- **Match-3 Mechanics**: Swap adjacent gems to create matches of 3 or more
- **Smooth Animations**: Falling gems and refill mechanics with smooth transitions
- **Scoring System**: Earn points for matches with bonuses for larger combos
- **Responsive UI**: Clean interface showing timer, scores, and game status

## Game Rules

1. **Objective**: Score more points than your opponent within 90 seconds
2. **How to Play**:
   - Click on a gem to select it
   - Click on an adjacent gem (horizontally or vertically) to swap
   - Match 3 or more gems of the same color to score points
   - Matched gems disappear and new gems fall from the top
3. **Scoring**:
   - 3 gems = 30 points
   - 4+ gems = 40+ points (bonus for larger matches)

## Building and Running

### Prerequisites
- Rust (latest stable version)
- Cargo

### Build Everything
```bash
cargo build --release
```

### Running the Multiplayer Game

#### 1. Start the Server
```bash
cargo run --release -p match3-server
```

The server will start on `127.0.0.1:9001`.

#### 2. Start Client(s)
```bash
# Terminal 1: Player 1
cargo run --release -p match3-pvp

# Terminal 2: Player 2
cargo run --release -p match3-pvp
```

Both clients will connect to the server, get matched, and play against each other!

### Running Standalone (Offline Mode)
```bash
cargo run --release -p match3-pvp
```

The client can also run standalone with simulated opponent for testing.

## Controls

- **Mouse/Touch**: Click to select and swap gems
- **Start Button**: Click to begin a new game
- **Play Again Button**: Click after game over to restart

## Technical Details

### Client
- **Language**: Rust
- **Framework**: Macrosquad 0.4
- **Grid Size**: 8x8
- **Gem Types**: 6 different colored gems
- **Platform**: Cross-platform (Desktop and Mobile)

### Server
- **Language**: Rust
- **Async Runtime**: Tokio
- **WebSocket**: tokio-tungstenite
- **Matchmaking**: FIFO queue system
- **Concurrency**: Supports multiple simultaneous games

### Protocol
- **Transport**: WebSocket (JSON messages)
- **Synchronization**: Real-time score and move updates
- **Timer**: Server-authoritative 90-second countdown

## Mobile Deployment

This game is built with Macrosquad which supports mobile platforms:

- **Android**: Use `cargo quad-apk` to build APK
- **iOS**: Use `cargo quad-ios` to build for iOS
- **WASM**: Build with `--target wasm32-unknown-unknown` for web

## Game Architecture

- **Match Detection**: Horizontal and vertical matching algorithm
- **Gravity System**: Gems fall to fill empty spaces
- **Animation System**: Smooth transitions for falling gems
- **State Management**: Menu, Playing, and Game Over states
- **Input Handling**: Mouse/touch support for gem selection and swapping

## Future Enhancements

- Network multiplayer (currently simulated)
- Power-ups and special gems
- Multiple game modes
- Sound effects and music
- Leaderboards
- Different difficulty levels

# Match3 PVP Server

Real-time multiplayer server for the Match3 PVP game. Handles player matchmaking, game synchronization, and score tracking.

## Features

- **WebSocket-based communication**: Real-time bidirectional communication
- **Automatic matchmaking**: Players are automatically paired when they join the queue
- **Game session management**: Each match is isolated with its own state
- **Score synchronization**: Real-time score updates between players
- **Timer management**: Server-side 90-second countdown for fair gameplay
- **Disconnect handling**: Gracefully handles player disconnections
- **Concurrent games**: Supports multiple simultaneous matches

## Architecture

### Components

1. **Connection Handler**: Manages WebSocket connections for each player
2. **Matchmaking Queue**: Pairs players in FIFO order
3. **Game Sessions**: Manages individual matches between two players
4. **State Manager**: Central state management for players, games, and queues

### Protocol

The server uses JSON-formatted messages over WebSocket:

#### Client → Server

- `JoinQueue`: Request to join matchmaking
- `SwapGems`: Notify server of gem swap (for opponent sync)
- `ScoreUpdate`: Send updated score to server
- `LeaveGame`: Leave current game

#### Server → Client

- `Connected`: Confirmation with player ID
- `Queued`: Position in matchmaking queue
- `MatchFound`: Opponent found, match starting
- `GameStarted`: Game begins
- `OpponentSwap`: Opponent made a move
- `ScoreUpdate`: Score update for both players
- `TimeUpdate`: Remaining time in seconds
- `GameOver`: Match ended with result (Win/Loss/Tie)
- `OpponentDisconnected`: Opponent left the game

## Running the Server

### Build

```bash
cargo build --release -p match3-server
```

### Run

```bash
cargo run --release -p match3-server
```

The server will listen on `127.0.0.1:9001` by default.

### Configuration

Edit `src/main.rs` to change:
- Server address (default: `127.0.0.1:9001`)
- Game duration (default: 90 seconds)

## Development

### Protocol Changes

The protocol is defined in the `protocol` crate, shared between client and server. Any changes should be made there to ensure consistency.

### Testing

Run multiple client instances to test matchmaking:

```bash
# Terminal 1: Start server
cargo run --release -p match3-server

# Terminal 2: Start client 1
cargo run --release -p match3-pvp

# Terminal 3: Start client 2
cargo run --release -p match3-pvp
```

## Technical Details

- **Language**: Rust
- **Async Runtime**: Tokio
- **WebSocket Library**: tokio-tungstenite
- **Serialization**: Serde JSON
- **Concurrency**: Arc, RwLock, Mutex for shared state

## Performance

- Handles multiple concurrent games efficiently
- Low-latency message passing
- Minimal overhead per connection

## Future Enhancements

- Authentication and player accounts
- Persistent leaderboards
- Replay system
- Anti-cheat validation
- Reconnection support
- Spectator mode
- Tournament brackets

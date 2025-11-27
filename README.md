# Brick City Wars - Advanced Match-3 PvP Battle Game

A complete real-time multiplayer PvP match-3 game built with Rust. Features advanced game mechanics including special gems, garbage attacks, booster loadouts, and strategic combo system.

## Project Structure

This is a Cargo workspace containing:

- **`client/`** - Game client with Macrosquad (desktop and mobile)
- **`server/`** - Multiplayer server with matchmaking and game synchronization
- **`protocol/`** - Shared protocol definitions for client-server communication

## Core Features

### Gameplay Mechanics
- **Real-time PvP**: Compete against an opponent to achieve the highest score
- **90-Second Matches**: Fast-paced gameplay with server-authoritative timer
- **Match-3 Mechanics**: Swap adjacent gems to create matches of 3 or more
- **Smooth Animations**: Falling gems and refill mechanics with smooth transitions
- **Screen Shake Effects**: Dynamic visual feedback for combos and attacks

### Special Gems & Power-ups
- **Drill (Match-4)**: Clears entire row or column
- **Barrel (L/T-Shape)**: 3x3 explosion radius
- **Mixer (Match-5+)**: Color bomb that clears all gems of one color
- **Special Combos**: Combine specials for devastating effects (e.g., Mixer+Mixer clears entire board)

### Strategic Systems
- **Energy Economy**: Build up energy (0-100) to activate boosters
- **Booster Loadout**: 3 active skills available during matches:
  - Micro-Refill (Key 1): +10 energy, cost 0
  - Garbage Push (Key 2): Convert bottom garbage row, cost 30
  - Barrel Burst (Key 3): Spawn random Barrel, cost 50
- **Garbage/Attack System**:
  - Send garbage to opponent with big matches
  - Garbage queues with 2.5-second warning before dropping
  - Cancel incoming garbage with Match-4+ or special activations
- **Overflow Loss**: Game ends if gems stack to the top row

### Multiplayer Features
- **Matchmaking**: FIFO queue system for fair pairing
- **Real-time Synchronization**: Score updates, swaps, and attacks
- **Rematch System**: Request rematches for continuous play
- **Disconnect Handling**: Graceful handling of opponent disconnects

## Game Rules

### Basic Gameplay
1. **Objective**: Score more points than your opponent within 90 seconds
2. **Matching**:
   - Click on a gem to select it
   - Click on an adjacent gem (horizontally or vertically) to swap
   - Match 3+ gems of the same color to score points
   - Matched gems disappear and new gems fall from the top

### Scoring & Special Creation
- **Match-3**: 30 points
- **Match-4**: 40 points + creates Drill gem + sends 1 garbage
- **Match-5+**: 50+ points + creates Mixer gem + sends 2 garbage
- **L/T Shape**: Creates Barrel gem + sends 2 garbage

### Special Gem Combos
- **Drill + Drill**: Cross clear (150 pts)
- **Drill + Barrel**: Row + 3x3 explosion (120 pts)
- **Barrel + Barrel**: 5x5 massive explosion (200 pts)
- **Mixer + Drill**: Convert color to Drills (250 pts)
- **Mixer + Barrel**: Convert color to Barrels (300 pts)
- **Mixer + Mixer**: MEGA CLEAR entire board (500 pts)

### Garbage System
- **Sending**: Create Match-4+ or activate specials to send garbage
- **Receiving**: Garbage queues for 2.5 seconds with warning
- **Cancellation**: Match-4+ or special activations cancel queued garbage
- **Penalty**: Unblocked garbage shifts your board up and adds garbage rows

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

The client can run standalone with simulated opponent for testing.

## Controls

### Mouse/Touch Controls
- **Click**: Select and swap gems
- **Double-tap Special**: Activate Drill, Barrel, or Mixer gems

### Keyboard Controls
- **Key 1**: Activate Micro-Refill booster
- **Key 2**: Activate Garbage Push booster
- **Key 3**: Activate Barrel Burst booster

### UI Buttons
- **Offline/Online**: Choose game mode from menu
- **Play Again**: Restart game (offline mode)
- **Request Rematch**: Request rematch (online mode)

## Technical Details

### Client
- **Language**: Rust
- **Framework**: Macrosquad 0.4
- **Grid Size**: 8x8
- **Gem Types**: 6 basic colors + 3 specials + garbage
- **Platform**: Cross-platform (Desktop and Mobile)
- **Visual Effects**: Screen shake, pulsing warnings, status indicators

### Server
- **Language**: Rust
- **Async Runtime**: Tokio
- **WebSocket**: tokio-tungstenite
- **Matchmaking**: FIFO queue system
- **Concurrency**: Supports multiple simultaneous games
- **Game Management**: Session-based with rematch support

### Protocol
- **Transport**: WebSocket (JSON messages)
- **Synchronization**: Real-time score, moves, garbage, and special activations
- **Timer**: Server-authoritative 90-second countdown
- **Rematch Flow**: Two-phase agreement system

## Mobile Deployment

This game is built with Macrosquad which supports mobile platforms:

- **Android**: Use `cargo quad-apk` to build APK
- **iOS**: Use `cargo quad-ios` to build for iOS
- **WASM**: Build with `--target wasm32-unknown-unknown` for web

## Game Architecture

### Core Systems
- **Match Detection**: Horizontal, vertical, and shape-based (L/T) matching
- **Gravity System**: Gems fall to fill empty spaces with smooth animation
- **Special Creation**: Automatic based on match size and shape
- **Combo Detection**: Pre-swap validation for special gem combinations
- **Energy System**: Accumulates from matches, caps at 100

### Advanced Mechanics
- **Garbage Queue**: Delayed application with cancellation window
- **Overflow Detection**: Checks row 0 for settled gems
- **Rematch State Machine**: Tracks mutual agreement for seamless rematches
- **Disconnect Recovery**: Handles network issues gracefully

### State Management
- Menu, Connecting, WaitingForMatch, Playing, GameOver states
- NetworkMode: Offline/Online with different behaviors
- Booster cooldown tracking (5-second reuse timer)

## Strategy Tips

1. **Prioritize Match-4+**: Creates specials and sends garbage
2. **Save Energy**: Use boosters strategically when under pressure
3. **Cancel Garbage**: Watch the incoming warning and make big matches
4. **Combo Specials**: Mixer combinations are devastating
5. **Watch Overflow**: Don't let garbage stack to the top
6. **Manage Resources**: Balance between attack and defense

## Feature Highlights

### Garbage Cancellation (Defense Mechanic)
- Incoming garbage shows pulsing red warning with countdown
- Match-4 cancels 2 garbage, Match-5+ cancels 4 garbage
- L/T shapes cancel 3 garbage
- Special activations also cancel garbage
- Visual feedback: "-X Incoming Blocked!"

### Rematch System (Seamless Continuation)
- Click "Request Rematch" after game ends
- Shows "Waiting for opponent..." or "Opponent wants rematch!"
- Automatic game reset when both players agree
- Handles opponent leaving gracefully

### Visual Polish
- Screen shake intensity scales with combo power
- Pulsing garbage warnings create urgency
- Energy bar with smooth fill animation
- Booster HUD with cooldown timers
- Win/Loss/Tie status with color coding

## Known Limitations

- Network communication is partially implemented (protocol ready, WebSocket connection TODO)
- Sound effects and music not yet implemented
- Particle effects planned but not implemented
- Floating combat text system planned but not implemented

## Future Enhancements

- Full WebSocket integration for online play
- Particle effects for special activations
- Floating combat text for score feedback
- Sound effects and music
- Leaderboards and ranking system
- Multiple game modes (Endless, Puzzle, etc.)
- Additional booster types
- Cosmetic gem skins

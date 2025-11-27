# Match3 PVP - Real-time Match-3 Battle Game

A real-time PvP match-3 game built with Rust and Macrosquad where players compete to get the most points in 90 seconds.

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

### Build
```bash
cargo build --release
```

### Run
```bash
cargo run --release
```

## Controls

- **Mouse/Touch**: Click to select and swap gems
- **Start Button**: Click to begin a new game
- **Play Again Button**: Click after game over to restart

## Technical Details

- **Language**: Rust
- **Framework**: Macrosquad 0.4
- **Grid Size**: 8x8
- **Gem Types**: 6 different colored gems
- **Platform**: Cross-platform (Desktop and Mobile)

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

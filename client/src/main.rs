use macroquad::prelude::*;
use ::rand::Rng;
use match3_protocol::{ClientMessage, ServerMessage, GameResult};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender, UnboundedReceiver};
use futures::{StreamExt, SinkExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};

const GRID_SIZE: usize = 8;
const GEM_SIZE: f32 = 60.0;
const BOARD_OFFSET_X: f32 = 50.0;
const BOARD_OFFSET_Y: f32 = 150.0;
const GAME_DURATION: f32 = 90.0;

// Extended gem types for Brick City Wars
#[derive(Clone, Copy, Debug, PartialEq)]
enum GemType {
    // Basic colors
    Red,
    Blue,
    Green,
    Yellow,
    Purple,
    Orange,
    // Special power-ups
    Drill,       // Match-4: Clears row or column
    Barrel,      // L/T shape: Explosion radius
    Mixer,       // Match-5: Color bomb
    Garbage,     // Unmatchable, blocks play
}

impl GemType {
    fn random_basic() -> Self {
        let mut rng = ::rand::thread_rng();
        match rng.gen_range(0..6) {
            0 => GemType::Red,
            1 => GemType::Blue,
            2 => GemType::Green,
            3 => GemType::Yellow,
            4 => GemType::Purple,
            _ => GemType::Orange,
        }
    }

    fn is_basic(&self) -> bool {
        matches!(self,
            GemType::Red | GemType::Blue | GemType::Green |
            GemType::Yellow | GemType::Purple | GemType::Orange
        )
    }

    fn is_special(&self) -> bool {
        matches!(self, GemType::Drill | GemType::Barrel | GemType::Mixer)
    }

    fn is_garbage(&self) -> bool {
        matches!(self, GemType::Garbage)
    }

    fn color(&self) -> Color {
        match self {
            GemType::Red => Color::from_rgba(255, 50, 50, 255),
            GemType::Blue => Color::from_rgba(50, 100, 255, 255),
            GemType::Green => Color::from_rgba(50, 255, 100, 255),
            GemType::Yellow => Color::from_rgba(255, 255, 50, 255),
            GemType::Purple => Color::from_rgba(200, 50, 255, 255),
            GemType::Orange => Color::from_rgba(255, 150, 50, 255),
            GemType::Drill => Color::from_rgba(150, 150, 150, 255),
            GemType::Barrel => Color::from_rgba(100, 50, 30, 255),
            GemType::Mixer => Color::from_rgba(255, 255, 255, 255),
            GemType::Garbage => Color::from_rgba(80, 80, 80, 255),
        }
    }
}

// Booster types for active skills
#[derive(Clone, Copy, Debug, PartialEq)]
enum BoosterType {
    MicroRefill,    // ID 0: +10 energy, cost 0
    GarbagePush,    // ID 1: Convert bottom garbage row, cost 30
    BarrelBurst,    // ID 2: Spawn random barrel, cost 50
}

impl BoosterType {
    fn id(&self) -> u8 {
        match self {
            BoosterType::MicroRefill => 0,
            BoosterType::GarbagePush => 1,
            BoosterType::BarrelBurst => 2,
        }
    }

    fn cost(&self) -> u32 {
        match self {
            BoosterType::MicroRefill => 0,  // Free for testing
            BoosterType::GarbagePush => 30,
            BoosterType::BarrelBurst => 50,
        }
    }

    fn name(&self) -> &str {
        match self {
            BoosterType::MicroRefill => "Refill",
            BoosterType::GarbagePush => "Push",
            BoosterType::BarrelBurst => "Barrel",
        }
    }

    fn color(&self) -> Color {
        match self {
            BoosterType::MicroRefill => Color::from_rgba(100, 255, 100, 255),
            BoosterType::GarbagePush => Color::from_rgba(255, 200, 100, 255),
            BoosterType::BarrelBurst => Color::from_rgba(200, 100, 255, 255),
        }
    }
}

#[derive(Clone, Copy)]
struct Booster {
    booster_type: BoosterType,
    cooldown_remaining: f32,
}

impl Booster {
    fn new(booster_type: BoosterType) -> Self {
        Self {
            booster_type,
            cooldown_remaining: 0.0,
        }
    }

    fn can_activate(&self, energy: u32) -> bool {
        self.cooldown_remaining <= 0.0 && energy >= self.booster_type.cost()
    }
}

#[derive(Clone, Copy)]
struct Gem {
    gem_type: GemType,
    y_offset: f32,
    is_falling: bool,
    marked_for_removal: bool,
}

impl Gem {
    fn new(gem_type: GemType) -> Self {
        Self {
            gem_type,
            y_offset: 0.0,
            is_falling: false,
            marked_for_removal: false,
        }
    }
}

#[derive(PartialEq, Clone)]
enum GameState {
    Menu,
    Login,
    Connecting,
    WaitingForMatch,
    Playing,
    GameOver,
    Leaderboard,
}

#[derive(PartialEq, Clone, Copy)]
enum NetworkMode {
    Offline,    // Simulated opponent
    Online,     // Real multiplayer
}

// Network bridge for async WebSocket communication
struct NetworkBridge {
    to_server: UnboundedSender<ClientMessage>,
    from_server: UnboundedReceiver<ServerMessage>,
}

impl NetworkBridge {
    fn send(&self, msg: ClientMessage) {
        let _ = self.to_server.send(msg);
    }

    fn try_recv(&mut self) -> Option<ServerMessage> {
        self.from_server.try_recv().ok()
    }
}

struct Game {
    grid: Vec<Vec<Option<Gem>>>,
    selected: Option<(usize, usize)>,
    score: u32,
    opponent_score: u32,
    energy: u32,
    state: GameState,
    time_remaining: f32,
    animation_timer: f32,
    network_mode: NetworkMode,
    pending_garbage: u8,
    garbage_queue: u8,         // Incoming garbage waiting to drop
    garbage_timer: f32,        // Time before queued garbage drops
    last_click_pos: Option<(usize, usize)>,
    last_click_time: f64,
    boosters: Vec<Booster>,
    shake_timer: f32,          // Screen shake effect
    requested_rematch: bool,   // Whether this player requested rematch
    opponent_requested_rematch: bool, // Whether opponent requested rematch
    disconnect_reason: Option<String>, // Reason for disconnect (if any)
    network_bridge: Option<NetworkBridge>, // WebSocket communication bridge
    // User account info
    username: String,          // Player's username
    pending_username: String,  // Username being typed in login screen
    elo: i32,                  // Player's ELO rating
    wins: u32,                 // Total wins
    losses: u32,               // Total losses
    bricks: u32,               // Currency: Bricks
    gold: u32,                 // Currency: Gold
    leaderboard_data: Vec<(String, i32)>, // Leaderboard data (username, elo)
    connecting_for_leaderboard: bool, // Flag to track if connecting for leaderboard
}

impl Game {
    fn new() -> Self {
        let mut game = Self {
            grid: vec![vec![None; GRID_SIZE]; GRID_SIZE],
            selected: None,
            score: 0,
            opponent_score: 0,
            energy: 0,
            state: GameState::Menu,
            time_remaining: GAME_DURATION,
            animation_timer: 0.0,
            network_mode: NetworkMode::Offline,
            pending_garbage: 0,
            garbage_queue: 0,
            garbage_timer: 0.0,
            last_click_pos: None,
            last_click_time: 0.0,
            boosters: vec![
                Booster::new(BoosterType::MicroRefill),
                Booster::new(BoosterType::GarbagePush),
                Booster::new(BoosterType::BarrelBurst),
            ],
            shake_timer: 0.0,
            requested_rematch: false,
            opponent_requested_rematch: false,
            disconnect_reason: None,
            network_bridge: None,
            username: String::new(),
            pending_username: String::new(),
            elo: 1000,
            wins: 0,
            losses: 0,
            bricks: 0,
            gold: 0,
            leaderboard_data: Vec::new(),
            connecting_for_leaderboard: false,
        };
        game.initialize_board();
        game
    }

    fn reset_game(&mut self) {
        // Reset game state for rematch
        self.grid = vec![vec![None; GRID_SIZE]; GRID_SIZE];
        self.selected = None;
        self.score = 0;
        self.opponent_score = 0;
        self.energy = 0;
        self.time_remaining = GAME_DURATION;
        self.animation_timer = 0.0;
        self.pending_garbage = 0;
        self.garbage_queue = 0;
        self.garbage_timer = 0.0;
        self.last_click_pos = None;
        self.last_click_time = 0.0;
        self.shake_timer = 0.0;
        self.requested_rematch = false;
        self.opponent_requested_rematch = false;
        self.disconnect_reason = None;

        // Reset booster cooldowns
        for booster in &mut self.boosters {
            booster.cooldown_remaining = 0.0;
        }

        self.initialize_board();
        self.state = GameState::Playing;
    }

    fn initialize_board(&mut self) {
        for row in 0..GRID_SIZE {
            for col in 0..GRID_SIZE {
                loop {
                    let gem = Gem::new(GemType::random_basic());
                    self.grid[row][col] = Some(gem);

                    if !self.would_create_initial_match(row, col) {
                        break;
                    }
                }
            }
        }
    }

    fn would_create_initial_match(&self, row: usize, col: usize) -> bool {
        if let Some(gem) = self.grid[row][col] {
            let gem_type = gem.gem_type;

            // Check horizontal
            let mut h_count = 1;
            if col >= 2 {
                if let Some(g) = self.grid[row][col - 1] {
                    if g.gem_type == gem_type {
                        h_count += 1;
                        if let Some(g2) = self.grid[row][col - 2] {
                            if g2.gem_type == gem_type {
                                h_count += 1;
                            }
                        }
                    }
                }
            }

            // Check vertical
            let mut v_count = 1;
            if row >= 2 {
                if let Some(g) = self.grid[row - 1][col] {
                    if g.gem_type == gem_type {
                        v_count += 1;
                        if let Some(g2) = self.grid[row - 2][col] {
                            if g2.gem_type == gem_type {
                                v_count += 1;
                            }
                        }
                    }
                }
            }

            h_count >= 3 || v_count >= 3
        } else {
            false
        }
    }

    fn start_game(&mut self, online: bool) {
        self.state = if online {
            self.network_mode = NetworkMode::Online;
            GameState::Connecting
        } else {
            self.network_mode = NetworkMode::Offline;
            GameState::Playing
        };

        self.score = 0;
        self.opponent_score = 0;
        self.energy = 0;
        self.time_remaining = GAME_DURATION;
        self.selected = None;
        self.pending_garbage = 0;
        self.garbage_queue = 0;
        self.garbage_timer = 0.0;
        self.initialize_board();
    }

    fn set_network_bridge(&mut self, bridge: NetworkBridge) {
        // Send JoinQueue message immediately after connection
        bridge.send(ClientMessage::JoinQueue);
        self.network_bridge = Some(bridge);
        self.state = GameState::WaitingForMatch;
    }

    fn update(&mut self, dt: f32) {
        match self.state {
            GameState::Playing => {
                self.time_remaining -= dt;
                if self.time_remaining <= 0.0 {
                    self.time_remaining = 0.0;
                    self.state = GameState::GameOver;
                }

                // Update booster cooldowns
                for booster in &mut self.boosters {
                    if booster.cooldown_remaining > 0.0 {
                        booster.cooldown_remaining -= dt;
                        if booster.cooldown_remaining < 0.0 {
                            booster.cooldown_remaining = 0.0;
                        }
                    }
                }

                // Update screen shake
                if self.shake_timer > 0.0 {
                    self.shake_timer -= dt;
                }

                // Update garbage queue timer
                if self.garbage_queue > 0 {
                    self.garbage_timer -= dt;
                    if self.garbage_timer <= 0.0 {
                        // Time's up - apply the queued garbage
                        self.pending_garbage = self.garbage_queue;
                        self.garbage_queue = 0;
                        self.garbage_timer = 0.0;
                    }
                }

                // Check for overflow (gems in row 0 or negative = instant loss)
                let mut has_overflow = false;
                for col in 0..GRID_SIZE {
                    if let Some(gem) = self.grid[0][col] {
                        if !gem.is_falling && gem.y_offset <= 0.0 {
                            // Check if it's a settled garbage or regular gem
                            if gem.gem_type.is_garbage() || gem.gem_type.is_basic() {
                                has_overflow = true;
                                break;
                            }
                        }
                    }
                }

                if has_overflow {
                    // Overflow = instant loss
                    self.state = GameState::GameOver;
                    self.time_remaining = 0.0;
                }

                // Update animations
                if self.animation_timer > 0.0 {
                    self.animation_timer -= dt;
                } else {
                    self.update_falling_gems(dt);

                    // Apply pending garbage
                    if self.pending_garbage > 0 {
                        self.apply_garbage();
                        self.pending_garbage = 0;
                    }
                }

                // Simulate opponent in offline mode
                if self.network_mode == NetworkMode::Offline {
                    if ::rand::random::<f32>() < 0.01 {
                        self.opponent_score += ::rand::thread_rng().gen_range(10..50);
                    }
                }
            }
            _ => {}
        }

        // Handle incoming network messages
        let mut messages = Vec::new();
        if let Some(bridge) = &mut self.network_bridge {
            while let Some(msg) = bridge.try_recv() {
                messages.push(msg);
            }
        }
        for msg in messages {
            self.handle_server_message(msg);
        }
    }

    fn handle_server_message(&mut self, msg: ServerMessage) {
        match msg {
            ServerMessage::AuthAccepted { player_id, username, elo, wins, losses, bricks, gold } => {
                println!("Authentication successful! Welcome {}", username);
                self.username = username;
                self.elo = elo;
                self.wins = wins;
                self.losses = losses;
                self.bricks = bricks;
                self.gold = gold;

                // Check if we're connecting for leaderboard or for playing
                if self.connecting_for_leaderboard {
                    // Request leaderboard data
                    if let Some(bridge) = &self.network_bridge {
                        bridge.send(ClientMessage::FetchLeaderboard);
                    }
                    self.connecting_for_leaderboard = false;
                } else {
                    // Join queue automatically after authentication
                    if let Some(bridge) = &self.network_bridge {
                        bridge.send(ClientMessage::JoinQueue);
                    }
                    self.state = GameState::WaitingForMatch;
                }
            }
            ServerMessage::AuthRejected { reason } => {
                println!("Authentication failed: {}", reason);
                self.disconnect_reason = Some(format!("Auth failed: {}", reason));
                self.state = GameState::Login;
            }
            ServerMessage::MatchResult { new_elo, elo_change, wins, losses, bricks, gold } => {
                self.elo = new_elo;
                self.wins = wins;
                self.losses = losses;
                self.bricks = bricks;
                self.gold = gold;
                println!("Match result: ELO {} ({:+}), W/L: {}/{}, Bricks: {}, Gold: {}",
                    new_elo, elo_change, wins, losses, bricks, gold);
            }
            ServerMessage::LeaderboardData { players } => {
                self.leaderboard_data = players;
                println!("Received leaderboard data with {} players", self.leaderboard_data.len());
                // Transition to leaderboard state when data is received
                self.state = GameState::Leaderboard;
            }
            ServerMessage::Connected { player_id } => {
                println!("Connected with player ID: {}", player_id);
            }
            ServerMessage::Queued { position } => {
                println!("In queue, position: {}", position);
            }
            ServerMessage::MatchFound { game_id, opponent_id } => {
                println!("Match found! Game ID: {}, Opponent: {}", game_id, opponent_id);
            }
            ServerMessage::GameStarted { game_id } => {
                println!("Game started! ID: {}", game_id);
                self.state = GameState::Playing;
                self.score = 0;
                self.opponent_score = 0;
                self.time_remaining = GAME_DURATION;
            }
            ServerMessage::OpponentSwap { row1, col1, row2, col2 } => {
                println!("Opponent swapped ({},{}) with ({},{})", row1, col1, row2, col2);
                // We don't visualize opponent's board, so just log it
            }
            ServerMessage::ScoreUpdate { player_score, opponent_score } => {
                self.score = player_score;
                self.opponent_score = opponent_score;
            }
            ServerMessage::TimeUpdate { seconds_remaining } => {
                self.time_remaining = seconds_remaining as f32;
            }
            ServerMessage::ReceiveGarbage { amount } => {
                self.receive_garbage(amount);
            }
            ServerMessage::OpponentActivatedSpecial { row, col } => {
                println!("Opponent activated special at ({},{})", row, col);
            }
            ServerMessage::OpponentActivatedBooster { booster_id } => {
                println!("Opponent activated booster #{}", booster_id);
            }
            ServerMessage::GameOver { winner } => {
                self.state = GameState::GameOver;
                self.time_remaining = 0.0;
                match winner {
                    GameResult::Win => println!("You won!"),
                    GameResult::Loss => println!("You lost!"),
                    GameResult::Tie => println!("It's a tie!"),
                }
            }
            ServerMessage::OpponentRequestedRematch => {
                self.handle_opponent_rematch_request();
            }
            ServerMessage::RematchAccepted => {
                self.handle_rematch_accepted();
            }
            ServerMessage::OpponentLeft => {
                self.handle_opponent_left();
            }
            ServerMessage::OpponentDisconnected => {
                self.handle_opponent_disconnected();
            }
            ServerMessage::Error { message } => {
                println!("Server error: {}", message);
            }
        }
    }

    fn update_falling_gems(&mut self, dt: f32) {
        let mut any_falling = false;

        for row in 0..GRID_SIZE {
            for col in 0..GRID_SIZE {
                if let Some(gem) = &mut self.grid[row][col] {
                    if gem.is_falling {
                        gem.y_offset -= 300.0 * dt;
                        if gem.y_offset <= 0.0 {
                            gem.y_offset = 0.0;
                            gem.is_falling = false;
                        } else {
                            any_falling = true;
                        }
                    }
                }
            }
        }

        if !any_falling {
            self.check_and_remove_matches();
        }
    }

    fn handle_click(&mut self, x: f32, y: f32) {
        if self.state != GameState::Playing || self.animation_timer > 0.0 {
            return;
        }

        let col = ((x - BOARD_OFFSET_X) / GEM_SIZE) as i32;
        let row = ((y - BOARD_OFFSET_Y) / GEM_SIZE) as i32;

        if col < 0 || col >= GRID_SIZE as i32 || row < 0 || row >= GRID_SIZE as i32 {
            self.selected = None;
            return;
        }

        let col = col as usize;
        let row = row as usize;

        // Check for double-tap on special gem
        let current_time = get_time();
        if let Some((last_row, last_col)) = self.last_click_pos {
            if last_row == row && last_col == col && (current_time - self.last_click_time) < 0.5 {
                // Double-tap detected - try to activate special
                if let Some(gem) = self.grid[row][col] {
                    if gem.gem_type.is_special() {
                        self.activate_special(row, col);
                        self.last_click_pos = None;
                        return;
                    }
                }
            }
        }

        self.last_click_pos = Some((row, col));
        self.last_click_time = current_time;

        if let Some((sel_row, sel_col)) = self.selected {
            let is_adjacent = (sel_row == row && (sel_col as i32 - col as i32).abs() == 1)
                || (sel_col == col && (sel_row as i32 - row as i32).abs() == 1);

            if is_adjacent {
                self.swap_gems(sel_row, sel_col, row, col);
                self.selected = None;
            } else {
                self.selected = Some((row, col));
            }
        } else {
            self.selected = Some((row, col));
        }
    }

    fn swap_gems(&mut self, row1: usize, col1: usize, row2: usize, col2: usize) {
        // Send swap message to server (online mode)
        if self.network_mode == NetworkMode::Online {
            if let Some(bridge) = &self.network_bridge {
                bridge.send(ClientMessage::SwapGems { row1, col1, row2, col2 });
            }
        }

        // Check for special gem combos BEFORE swapping
        let gem1 = self.grid[row1][col1];
        let gem2 = self.grid[row2][col2];

        if let (Some(g1), Some(g2)) = (gem1, gem2) {
            if g1.gem_type.is_special() && g2.gem_type.is_special() {
                // Special + Special = COMBO!
                self.activate_combo(row1, col1, row2, col2, g1.gem_type, g2.gem_type);
                return;
            }
        }

        let temp = self.grid[row1][col1];
        self.grid[row1][col1] = self.grid[row2][col2];
        self.grid[row2][col2] = temp;

        let has_match = self.has_match_at(row1, col1) || self.has_match_at(row2, col2);

        if has_match {
            self.animation_timer = 0.3;
            self.check_and_remove_matches();
        } else {
            // Swap back if no match
            let temp = self.grid[row1][col1];
            self.grid[row1][col1] = self.grid[row2][col2];
            self.grid[row2][col2] = temp;
        }
    }

    fn has_match_at(&self, row: usize, col: usize) -> bool {
        if self.grid[row][col].is_none() {
            return false;
        }

        let gem = self.grid[row][col].unwrap();
        if !gem.gem_type.is_basic() {
            return false;
        }

        let gem_type = gem.gem_type;

        // Check horizontal
        let mut h_count = 1;
        let mut c = col as i32 - 1;
        while c >= 0 {
            if let Some(g) = self.grid[row][c as usize] {
                if g.gem_type == gem_type {
                    h_count += 1;
                    c -= 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        let mut c = col + 1;
        while c < GRID_SIZE {
            if let Some(g) = self.grid[row][c] {
                if g.gem_type == gem_type {
                    h_count += 1;
                    c += 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        if h_count >= 3 {
            return true;
        }

        // Check vertical
        let mut v_count = 1;
        let mut r = row as i32 - 1;
        while r >= 0 {
            if let Some(g) = self.grid[r as usize][col] {
                if g.gem_type == gem_type {
                    v_count += 1;
                    r -= 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        let mut r = row + 1;
        while r < GRID_SIZE {
            if let Some(g) = self.grid[r][col] {
                if g.gem_type == gem_type {
                    v_count += 1;
                    r += 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        v_count >= 3
    }

    fn check_and_remove_matches(&mut self) {
        let matches = self.find_all_matches();

        if matches.is_empty() {
            return;
        }

        // Calculate energy and garbage from matches
        let total_gems = matches.len();
        let mut garbage_to_send = 0;
        let mut garbage_cancelled = 0;

        // Mark gems for removal and create specials
        for match_info in &matches {
            match match_info {
                MatchType::Line(positions) => {
                    for &(r, c) in positions {
                        if let Some(gem) = &mut self.grid[r][c] {
                            gem.marked_for_removal = true;
                        }
                    }

                    // Create special based on match size
                    if positions.len() == 4 {
                        // Match-4: Create Drill
                        let (r, c) = positions[positions.len() / 2];
                        self.grid[r][c] = Some(Gem::new(GemType::Drill));
                        garbage_to_send += 1;
                        garbage_cancelled += 2; // Cancel 2 incoming garbage
                    } else if positions.len() >= 5 {
                        // Match-5+: Create Mixer
                        let (r, c) = positions[positions.len() / 2];
                        self.grid[r][c] = Some(Gem::new(GemType::Mixer));
                        garbage_to_send += 2;
                        garbage_cancelled += 4; // Cancel 4 incoming garbage
                    }
                }
                MatchType::LShape(positions) | MatchType::TShape(positions) => {
                    for &(r, c) in positions {
                        if let Some(gem) = &mut self.grid[r][c] {
                            gem.marked_for_removal = true;
                        }
                    }
                    // L/T Shape: Create Barrel
                    let (r, c) = positions[positions.len() / 2];
                    self.grid[r][c] = Some(Gem::new(GemType::Barrel));
                    garbage_to_send += 2;
                    garbage_cancelled += 3; // Cancel 3 incoming garbage
                }
            }
        }

        // Remove marked gems
        for row in 0..GRID_SIZE {
            for col in 0..GRID_SIZE {
                if let Some(gem) = self.grid[row][col] {
                    if gem.marked_for_removal {
                        self.grid[row][col] = None;
                    }
                }
            }
        }

        // Update score and energy
        self.score += total_gems as u32 * 10;
        self.energy = (self.energy + total_gems as u32).min(100);

        // Apply garbage cancellation
        if garbage_cancelled > 0 && self.garbage_queue > 0 {
            let actually_cancelled = garbage_cancelled.min(self.garbage_queue as u32);
            self.garbage_queue = self.garbage_queue.saturating_sub(actually_cancelled as u8);

            // Visual feedback for cancellation
            if actually_cancelled > 0 {
                println!("-{} Incoming Blocked!", actually_cancelled);
                self.shake_timer = 0.2; // Small shake for feedback
            }

            // Reset garbage timer if queue is now empty
            if self.garbage_queue == 0 {
                self.garbage_timer = 0.0;
            }
        }

        // Send garbage to opponent
        if garbage_to_send > 0 && self.network_mode == NetworkMode::Online {
            if let Some(bridge) = &self.network_bridge {
                bridge.send(ClientMessage::SendGarbage { amount: garbage_to_send });
            }
        }

        if total_gems >= 4 {
            self.score += 20;
        }

        // Send score update to server
        if self.network_mode == NetworkMode::Online {
            if let Some(bridge) = &self.network_bridge {
                bridge.send(ClientMessage::ScoreUpdate { score: self.score });
            }
        }

        self.apply_gravity();
    }

    fn find_all_matches(&self) -> Vec<MatchType> {
        let mut matches = Vec::new();
        let mut processed = vec![vec![false; GRID_SIZE]; GRID_SIZE];

        // Find L and T shapes first (they're more specific)
        for row in 0..GRID_SIZE {
            for col in 0..GRID_SIZE {
                if processed[row][col] {
                    continue;
                }

                if let Some(gem) = self.grid[row][col] {
                    if !gem.gem_type.is_basic() {
                        continue;
                    }

                    // Check for L/T shapes
                    if let Some(shape_match) = self.find_shape_match(row, col) {
                        for &(r, c) in shape_match.positions() {
                            processed[r][c] = true;
                        }
                        matches.push(shape_match);
                    }
                }
            }
        }

        // Find line matches
        for row in 0..GRID_SIZE {
            for col in 0..GRID_SIZE {
                if processed[row][col] {
                    continue;
                }

                if let Some(gem) = self.grid[row][col] {
                    if !gem.gem_type.is_basic() {
                        continue;
                    }

                    let gem_type = gem.gem_type;

                    // Horizontal matches
                    let mut h_positions = vec![(row, col)];
                    for c in (col + 1)..GRID_SIZE {
                        if processed[row][c] {
                            break;
                        }
                        if let Some(g) = self.grid[row][c] {
                            if g.gem_type == gem_type {
                                h_positions.push((row, c));
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }

                    if h_positions.len() >= 3 {
                        for &(r, c) in &h_positions {
                            processed[r][c] = true;
                        }
                        matches.push(MatchType::Line(h_positions));
                        continue;
                    }

                    // Vertical matches
                    let mut v_positions = vec![(row, col)];
                    for r in (row + 1)..GRID_SIZE {
                        if processed[r][col] {
                            break;
                        }
                        if let Some(g) = self.grid[r][col] {
                            if g.gem_type == gem_type {
                                v_positions.push((r, col));
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }

                    if v_positions.len() >= 3 {
                        for &(r, c) in &v_positions {
                            processed[r][c] = true;
                        }
                        matches.push(MatchType::Line(v_positions));
                    }
                }
            }
        }

        matches
    }

    fn find_shape_match(&self, row: usize, col: usize) -> Option<MatchType> {
        if let Some(gem) = self.grid[row][col] {
            if !gem.gem_type.is_basic() {
                return None;
            }

            let gem_type = gem.gem_type;

            // Check L and T shapes (need at least 5 gems total)
            // L shape patterns: horizontal line + vertical extension
            // T shape patterns: vertical line + horizontal extension

            // Try L shapes (4 patterns)
            // Pattern 1: ─┐ (horizontal right, vertical down)
            if col + 2 < GRID_SIZE && row + 2 < GRID_SIZE {
                if self.check_gem_type(row, col + 1, gem_type) &&
                   self.check_gem_type(row, col + 2, gem_type) &&
                   self.check_gem_type(row + 1, col + 2, gem_type) &&
                   self.check_gem_type(row + 2, col + 2, gem_type) {
                    return Some(MatchType::LShape(vec![
                        (row, col), (row, col + 1), (row, col + 2),
                        (row + 1, col + 2), (row + 2, col + 2)
                    ]));
                }
            }

            // More L/T shape patterns can be added here...
        }

        None
    }

    fn check_gem_type(&self, row: usize, col: usize, gem_type: GemType) -> bool {
        if let Some(gem) = self.grid[row][col] {
            gem.gem_type == gem_type
        } else {
            false
        }
    }

    fn activate_special(&mut self, row: usize, col: usize) {
        // Send activation message to server
        if self.network_mode == NetworkMode::Online {
            if let Some(bridge) = &self.network_bridge {
                bridge.send(ClientMessage::ActivateSpecial { row, col });
            }
        }

        if let Some(gem) = self.grid[row][col] {
            match gem.gem_type {
                GemType::Drill => {
                    self.activate_drill(row, col);
                }
                GemType::Barrel => {
                    self.activate_barrel(row, col);
                }
                GemType::Mixer => {
                    self.activate_mixer(row, col);
                }
                _ => {}
            }
        }
    }

    fn activate_drill(&mut self, row: usize, col: usize) {
        // Clear entire row and column
        for c in 0..GRID_SIZE {
            self.grid[row][c] = None;
        }
        for r in 0..GRID_SIZE {
            self.grid[r][col] = None;
        }

        self.score += 50;

        // Cancel garbage from queue
        if self.garbage_queue > 0 {
            let cancelled = 1u8.min(self.garbage_queue);
            self.garbage_queue -= cancelled;
            println!("-{} Incoming Blocked!", cancelled);
            if self.garbage_queue == 0 {
                self.garbage_timer = 0.0;
            }
        }

        self.apply_gravity();
        self.animation_timer = 0.3;
    }

    fn activate_barrel(&mut self, row: usize, col: usize) {
        // Clear 3x3 area
        let min_row = row.saturating_sub(1);
        let max_row = (row + 2).min(GRID_SIZE);
        let min_col = col.saturating_sub(1);
        let max_col = (col + 2).min(GRID_SIZE);

        for r in min_row..max_row {
            for c in min_col..max_col {
                self.grid[r][c] = None;
            }
        }

        self.score += 40;

        // Cancel garbage from queue
        if self.garbage_queue > 0 {
            let cancelled = 2u8.min(self.garbage_queue);
            self.garbage_queue -= cancelled;
            println!("-{} Incoming Blocked!", cancelled);
            if self.garbage_queue == 0 {
                self.garbage_timer = 0.0;
            }
        }

        self.apply_gravity();
        self.animation_timer = 0.3;
    }

    fn activate_mixer(&mut self, row: usize, col: usize) {
        // Remove all gems of a random color
        let target_color = GemType::random_basic();

        for r in 0..GRID_SIZE {
            for c in 0..GRID_SIZE {
                if let Some(gem) = self.grid[r][c] {
                    if gem.gem_type == target_color {
                        self.grid[r][c] = None;
                    }
                }
            }
        }

        self.grid[row][col] = None;
        self.score += 100;

        // Cancel garbage from queue
        if self.garbage_queue > 0 {
            let cancelled = 3u8.min(self.garbage_queue);
            self.garbage_queue -= cancelled;
            println!("-{} Incoming Blocked!", cancelled);
            if self.garbage_queue == 0 {
                self.garbage_timer = 0.0;
            }
        }

        self.apply_gravity();
        self.animation_timer = 0.3;
    }

    fn activate_combo(&mut self, row1: usize, col1: usize, row2: usize, col2: usize, type1: GemType, type2: GemType) {
        // Remove both gems first
        self.grid[row1][col1] = None;
        self.grid[row2][col2] = None;

        // Determine combo type
        match (type1, type2) {
            (GemType::Drill, GemType::Drill) => {
                // Cross Clear: Both row AND column of impact point
                let center_row = (row1 + row2) / 2;
                let center_col = (col1 + col2) / 2;

                // Clear entire row
                for c in 0..GRID_SIZE {
                    self.grid[center_row][c] = None;
                }
                // Clear entire column
                for r in 0..GRID_SIZE {
                    self.grid[r][center_col] = None;
                }

                self.score += 150;
                self.shake_timer = 0.4; // Medium shake
            }
            (GemType::Drill, GemType::Barrel) | (GemType::Barrel, GemType::Drill) => {
                // Row clear + 3x3 explosion at center
                let center_row = (row1 + row2) / 2;
                let center_col = (col1 + col2) / 2;

                // Clear row
                for c in 0..GRID_SIZE {
                    self.grid[center_row][c] = None;
                }

                // 3x3 explosion
                let min_row = center_row.saturating_sub(1);
                let max_row = (center_row + 2).min(GRID_SIZE);
                let min_col = center_col.saturating_sub(1);
                let max_col = (center_col + 2).min(GRID_SIZE);

                for r in min_row..max_row {
                    for c in min_col..max_col {
                        self.grid[r][c] = None;
                    }
                }

                self.score += 120;
                self.shake_timer = 0.35; // Medium shake
            }
            (GemType::Barrel, GemType::Barrel) => {
                // Massive 5x5 explosion
                let center_row = (row1 + row2) / 2;
                let center_col = (col1 + col2) / 2;

                let min_row = center_row.saturating_sub(2);
                let max_row = (center_row + 3).min(GRID_SIZE);
                let min_col = center_col.saturating_sub(2);
                let max_col = (center_col + 3).min(GRID_SIZE);

                for r in min_row..max_row {
                    for c in min_col..max_col {
                        self.grid[r][c] = None;
                    }
                }

                self.score += 200;
                self.shake_timer = 0.5; // Strong shake
            }
            (GemType::Mixer, GemType::Drill) | (GemType::Drill, GemType::Mixer) => {
                // Convert all gems of one color to Drills
                let target_color = GemType::random_basic();

                for r in 0..GRID_SIZE {
                    for c in 0..GRID_SIZE {
                        if let Some(gem) = self.grid[r][c] {
                            if gem.gem_type == target_color {
                                self.grid[r][c] = Some(Gem::new(GemType::Drill));
                            }
                        }
                    }
                }

                self.score += 250;
                self.shake_timer = 0.6; // Strong shake
            }
            (GemType::Mixer, GemType::Barrel) | (GemType::Barrel, GemType::Mixer) => {
                // Convert all gems of one color to Barrels
                let target_color = GemType::random_basic();

                for r in 0..GRID_SIZE {
                    for c in 0..GRID_SIZE {
                        if let Some(gem) = self.grid[r][c] {
                            if gem.gem_type == target_color {
                                self.grid[r][c] = Some(Gem::new(GemType::Barrel));
                            }
                        }
                    }
                }

                self.score += 300;
                self.shake_timer = 0.7; // Very strong shake
            }
            (GemType::Mixer, GemType::Mixer) => {
                // MEGA CLEAR: Clear entire board!
                for r in 0..GRID_SIZE {
                    for c in 0..GRID_SIZE {
                        self.grid[r][c] = None;
                    }
                }

                self.score += 500;
                self.shake_timer = 1.0; // MEGA shake!
            }
            _ => {} // Shouldn't happen
        }

        self.apply_gravity();
        self.animation_timer = 0.3;
    }

    fn receive_garbage(&mut self, amount: u8) {
        // Add to queue instead of applying immediately
        self.garbage_queue = self.garbage_queue.saturating_add(amount);

        // Reset timer to 2.5 seconds (warning phase)
        self.garbage_timer = 2.5;

        // Visual feedback
        println!("{} Incoming Garbage!", amount);
        self.shake_timer = 0.3;
    }

    fn handle_opponent_disconnected(&mut self) {
        self.disconnect_reason = Some("Opponent Disconnected - You Win!".to_string());
        self.state = GameState::GameOver;
        self.time_remaining = 0.0;
    }

    fn handle_opponent_left(&mut self) {
        self.disconnect_reason = Some("Opponent Left - You Win!".to_string());
        self.state = GameState::GameOver;
        self.time_remaining = 0.0;
    }

    fn handle_opponent_rematch_request(&mut self) {
        self.opponent_requested_rematch = true;
    }

    fn handle_rematch_accepted(&mut self) {
        // Both players agreed to rematch - reset and start new game
        self.reset_game();
    }

    fn activate_booster(&mut self, booster_index: usize) {
        if booster_index >= self.boosters.len() {
            return;
        }

        let booster = self.boosters[booster_index];
        if !booster.can_activate(self.energy) {
            return;
        }

        // Deduct energy cost
        self.energy = self.energy.saturating_sub(booster.booster_type.cost());

        // Activate booster effect
        match booster.booster_type {
            BoosterType::MicroRefill => {
                self.energy = (self.energy + 10).min(100);
            }
            BoosterType::GarbagePush => {
                // Convert bottom garbage row to random gems
                for col in 0..GRID_SIZE {
                    if let Some(gem) = self.grid[GRID_SIZE - 1][col] {
                        if gem.gem_type.is_garbage() {
                            self.grid[GRID_SIZE - 1][col] = Some(Gem::new(GemType::random_basic()));
                        }
                    }
                }
            }
            BoosterType::BarrelBurst => {
                // Spawn a random Barrel on the board
                let mut rng = ::rand::thread_rng();
                let row = rng.gen_range(0..GRID_SIZE);
                let col = rng.gen_range(0..GRID_SIZE);
                self.grid[row][col] = Some(Gem::new(GemType::Barrel));
            }
        }

        // Set cooldown
        self.boosters[booster_index].cooldown_remaining = 5.0;

        // Send network message if online
        if self.network_mode == NetworkMode::Online {
            if let Some(bridge) = &self.network_bridge {
                bridge.send(ClientMessage::ActivateBooster { booster_id: booster.booster_type.id() });
            }
        }
    }

    fn apply_garbage(&mut self) {
        let amount = self.pending_garbage as usize;

        // Shift existing rows up
        for _ in 0..amount {
            // Move all rows up
            for row in 0..GRID_SIZE - 1 {
                for col in 0..GRID_SIZE {
                    self.grid[row][col] = self.grid[row + 1][col];
                }
            }

            // Fill bottom row with garbage
            for col in 0..GRID_SIZE {
                self.grid[GRID_SIZE - 1][col] = Some(Gem::new(GemType::Garbage));
            }
        }
    }

    fn apply_gravity(&mut self) {
        for col in 0..GRID_SIZE {
            let mut write_row = GRID_SIZE;

            for row in (0..GRID_SIZE).rev() {
                if self.grid[row][col].is_some() {
                    write_row -= 1;
                    if write_row != row {
                        self.grid[write_row][col] = self.grid[row][col];
                        self.grid[row][col] = None;
                    }
                }
            }

            for row in 0..write_row {
                let mut new_gem = Gem::new(GemType::random_basic());
                new_gem.y_offset = (write_row - row) as f32 * GEM_SIZE;
                new_gem.is_falling = true;
                self.grid[row][col] = Some(new_gem);
            }
        }

        self.animation_timer = 0.3;
    }

    fn draw(&self) {
        clear_background(Color::from_rgba(20, 20, 40, 255));

        match self.state {
            GameState::Menu => self.draw_menu(),
            GameState::Login => self.draw_login(),
            GameState::Connecting => self.draw_connecting(),
            GameState::WaitingForMatch => self.draw_waiting(),
            GameState::Playing => self.draw_game(),
            GameState::GameOver => self.draw_game_over(),
            GameState::Leaderboard => self.draw_leaderboard(),
        }
    }

    fn draw_menu(&self) {
        let screen_width = screen_width();
        let screen_height = screen_height();

        draw_text(
            "BRICK CITY WARS",
            screen_width / 2.0 - 180.0,
            screen_height / 2.0 - 150.0,
            60.0,
            WHITE,
        );

        draw_text(
            "Match-3 PvP Battle",
            screen_width / 2.0 - 100.0,
            screen_height / 2.0 - 90.0,
            25.0,
            LIGHTGRAY,
        );

        // Offline button
        let offline_x = screen_width / 2.0 - 100.0;
        let offline_y = screen_height / 2.0 - 20.0;
        draw_rectangle(offline_x, offline_y, 200.0, 50.0, GREEN);
        draw_text("OFFLINE MODE", offline_x + 20.0, offline_y + 33.0, 25.0, WHITE);

        // Online button
        let online_x = screen_width / 2.0 - 100.0;
        let online_y = screen_height / 2.0 + 50.0;
        draw_rectangle(online_x, online_y, 200.0, 50.0, BLUE);
        draw_text("ONLINE MODE", online_x + 25.0, online_y + 33.0, 25.0, WHITE);

        // Leaderboard button
        let leaderboard_x = screen_width / 2.0 - 100.0;
        let leaderboard_y = screen_height / 2.0 + 120.0;
        draw_rectangle(leaderboard_x, leaderboard_y, 200.0, 50.0, Color::from_rgba(200, 100, 255, 255));
        draw_text("LEADERBOARD", leaderboard_x + 25.0, leaderboard_y + 33.0, 25.0, WHITE);

        // Instructions
        draw_text(
            "Match 3+ gems | Double-tap specials",
            screen_width / 2.0 - 150.0,
            screen_height / 2.0 + 190.0,
            18.0,
            LIGHTGRAY,
        );
    }

    fn draw_login(&self) {
        let screen_width = screen_width();
        let screen_height = screen_height();

        draw_text(
            "ENTER USERNAME",
            screen_width / 2.0 - 130.0,
            screen_height / 2.0 - 100.0,
            40.0,
            WHITE,
        );

        // Display error message if there is one
        if let Some(ref reason) = self.disconnect_reason {
            draw_text(
                reason,
                screen_width / 2.0 - 150.0,
                screen_height / 2.0 - 50.0,
                20.0,
                RED,
            );
        }

        // Username input box
        let input_x = screen_width / 2.0 - 150.0;
        let input_y = screen_height / 2.0 - 30.0;
        let input_width = 300.0;
        let input_height = 50.0;

        draw_rectangle(input_x, input_y, input_width, input_height, DARKGRAY);
        draw_rectangle_lines(input_x, input_y, input_width, input_height, 2.0, WHITE);

        // Display username being typed
        let display_text = if self.pending_username.is_empty() {
            "Type your username..."
        } else {
            &self.pending_username
        };
        let text_color = if self.pending_username.is_empty() { GRAY } else { WHITE };
        draw_text(display_text, input_x + 10.0, input_y + 33.0, 25.0, text_color);

        // Continue button (only enabled if username is not empty)
        let button_x = screen_width / 2.0 - 100.0;
        let button_y = screen_height / 2.0 + 50.0;
        let button_enabled = !self.pending_username.is_empty();
        let button_color = if button_enabled { GREEN } else { GRAY };

        draw_rectangle(button_x, button_y, 200.0, 50.0, button_color);
        draw_text("CONTINUE", button_x + 40.0, button_y + 33.0, 25.0, WHITE);

        // Instructions
        draw_text(
            "Press ENTER or click CONTINUE",
            screen_width / 2.0 - 140.0,
            screen_height / 2.0 + 130.0,
            18.0,
            LIGHTGRAY,
        );

        // Back button
        let back_x = screen_width / 2.0 - 100.0;
        let back_y = screen_height / 2.0 + 170.0;
        draw_rectangle(back_x, back_y, 200.0, 40.0, Color::from_rgba(100, 100, 100, 255));
        draw_text("BACK", back_x + 75.0, back_y + 27.0, 20.0, WHITE);
    }

    fn draw_connecting(&self) {
        let screen_width = screen_width();
        let screen_height = screen_height();

        draw_text(
            "Connecting to server...",
            screen_width / 2.0 - 150.0,
            screen_height / 2.0,
            30.0,
            WHITE,
        );
    }

    fn draw_waiting(&self) {
        let screen_width = screen_width();
        let screen_height = screen_height();

        draw_text(
            "Waiting for opponent...",
            screen_width / 2.0 - 150.0,
            screen_height / 2.0,
            30.0,
            WHITE,
        );
    }

    fn draw_game(&self) {
        // Header background
        draw_rectangle(0.0, 0.0, screen_width(), 130.0, Color::from_rgba(30, 30, 60, 255));

        // Timer
        let minutes = (self.time_remaining / 60.0) as u32;
        let seconds = (self.time_remaining % 60.0) as u32;
        let timer_text = format!("Time: {:02}:{:02}", minutes, seconds);
        let timer_color = if self.time_remaining < 20.0 { RED } else { WHITE };
        draw_text(&timer_text, 20.0, 35.0, 35.0, timer_color);

        // Scores
        draw_text(
            &format!("You: {}", self.score),
            20.0, 70.0, 28.0, YELLOW,
        );

        draw_text(
            &format!("Opponent: {}", self.opponent_score),
            screen_width() - 230.0, 35.0, 28.0,
            Color::from_rgba(255, 100, 100, 255),
        );

        // Energy bar
        let energy_x = 20.0;
        let energy_y = 95.0;
        let energy_width = 200.0;
        let energy_height = 20.0;

        draw_rectangle(energy_x, energy_y, energy_width, energy_height, Color::from_rgba(50, 50, 50, 255));
        let filled_width = (self.energy as f32 / 100.0) * energy_width;
        draw_rectangle(energy_x, energy_y, filled_width, energy_height, Color::from_rgba(255, 200, 0, 255));
        draw_rectangle_lines(energy_x, energy_y, energy_width, energy_height, 2.0, WHITE);
        draw_text(&format!("Energy: {}/100", self.energy), energy_x, energy_y - 5.0, 18.0, WHITE);

        // Resource HUD (Bricks and Gold) - displayed next to energy
        let resource_x = energy_x + energy_width + 30.0;
        let resource_y = 95.0;

        // Bricks icon and count
        draw_rectangle(resource_x, resource_y, 25.0, 20.0, Color::from_rgba(180, 100, 50, 255));
        draw_text(
            &format!("{}", self.bricks),
            resource_x + 30.0, resource_y + 16.0,
            20.0, Color::from_rgba(255, 180, 100, 255)
        );

        // Gold icon and count
        let gold_x = resource_x + 100.0;
        draw_circle(gold_x + 12.0, resource_y + 10.0, 10.0, Color::from_rgba(255, 215, 0, 255));
        draw_text(
            &format!("{}", self.gold),
            gold_x + 30.0, resource_y + 16.0,
            20.0, Color::from_rgba(255, 215, 0, 255)
        );

        // Booster HUD (right side)
        let booster_start_x = screen_width() - 250.0;
        let booster_y = 95.0;
        let booster_size = 50.0;
        let booster_spacing = 60.0;

        for (i, booster) in self.boosters.iter().enumerate() {
            let x = booster_start_x + i as f32 * booster_spacing;
            let can_use = booster.can_activate(self.energy);

            // Draw booster box
            let box_color = if can_use {
                booster.booster_type.color()
            } else {
                Color::from_rgba(60, 60, 60, 255)
            };

            draw_rectangle(x, booster_y, booster_size, booster_size, box_color);
            draw_rectangle_lines(x, booster_y, booster_size, booster_size, 2.0, WHITE);

            // Draw booster name
            draw_text(
                booster.booster_type.name(),
                x + 5.0,
                booster_y + 25.0,
                20.0,
                WHITE
            );

            // Draw cost
            draw_text(
                &format!("{}", booster.booster_type.cost()),
                x + 15.0,
                booster_y + 45.0,
                16.0,
                YELLOW
            );

            // Draw key hint
            draw_text(
                &format!("[{}]", i + 1),
                x + 5.0,
                booster_y - 3.0,
                14.0,
                LIGHTGRAY
            );

            // Draw cooldown if active
            if booster.cooldown_remaining > 0.0 {
                let cd_text = format!("{:.1}s", booster.cooldown_remaining);
                draw_text(&cd_text, x + 10.0, booster_y + 35.0, 18.0, RED);
            }
        }

        // Status indicator
        if self.score > self.opponent_score {
            draw_text("WINNING!", screen_width() - 230.0, 70.0, 25.0, GREEN);
        } else if self.score < self.opponent_score {
            draw_text("LOSING!", screen_width() - 230.0, 70.0, 25.0, RED);
        } else {
            draw_text("TIED!", screen_width() - 230.0, 70.0, 25.0, YELLOW);
        }

        // Garbage queue warning
        if self.garbage_queue > 0 {
            let warning_text = format!("⚠ {} INCOMING GARBAGE!", self.garbage_queue);
            let warning_x = screen_width() / 2.0 - 150.0;
            let warning_y = 120.0;

            // Pulsing effect based on timer
            let pulse = (self.garbage_timer * 3.0).sin().abs();
            let alpha = (150.0 + pulse * 105.0) as u8;

            draw_rectangle(
                warning_x - 10.0, warning_y - 30.0,
                320.0, 40.0,
                Color::from_rgba(200, 0, 0, alpha)
            );
            draw_text(&warning_text, warning_x, warning_y, 30.0, WHITE);

            // Time remaining until drop
            let time_text = format!("{:.1}s", self.garbage_timer);
            draw_text(&time_text, warning_x + 250.0, warning_y, 25.0, YELLOW);
        }

        // Calculate screen shake offset
        let shake_x = if self.shake_timer > 0.0 {
            let intensity = 8.0;
            ((self.shake_timer * 50.0).sin() * intensity) as f32
        } else {
            0.0
        };
        let shake_y = if self.shake_timer > 0.0 {
            let intensity = 8.0;
            ((self.shake_timer * 60.0).cos() * intensity) as f32
        } else {
            0.0
        };

        // Grid
        draw_rectangle(
            BOARD_OFFSET_X - 10.0 + shake_x,
            BOARD_OFFSET_Y - 10.0 + shake_y,
            GRID_SIZE as f32 * GEM_SIZE + 20.0,
            GRID_SIZE as f32 * GEM_SIZE + 20.0,
            Color::from_rgba(40, 40, 70, 255),
        );

        // Gems
        for row in 0..GRID_SIZE {
            for col in 0..GRID_SIZE {
                let x = BOARD_OFFSET_X + col as f32 * GEM_SIZE + shake_x;
                let y = BOARD_OFFSET_Y + row as f32 * GEM_SIZE + shake_y;

                draw_rectangle(
                    x + 2.0, y + 2.0,
                    GEM_SIZE - 4.0, GEM_SIZE - 4.0,
                    Color::from_rgba(30, 30, 50, 255),
                );

                if let Some(gem) = self.grid[row][col] {
                    let gem_y = y + gem.y_offset;

                    // Draw gem based on type
                    match gem.gem_type {
                        GemType::Red | GemType::Blue | GemType::Green |
                        GemType::Yellow | GemType::Purple | GemType::Orange => {
                            draw_circle(
                                x + GEM_SIZE / 2.0,
                                gem_y + GEM_SIZE / 2.0,
                                GEM_SIZE / 2.5,
                                gem.gem_type.color(),
                            );
                            draw_circle(
                                x + GEM_SIZE / 2.0 - 8.0,
                                gem_y + GEM_SIZE / 2.0 - 8.0,
                                GEM_SIZE / 8.0,
                                Color::from_rgba(255, 255, 255, 150),
                            );
                        }
                        GemType::Drill => {
                            draw_rectangle(
                                x + 10.0, gem_y + 10.0,
                                GEM_SIZE - 20.0, GEM_SIZE - 20.0,
                                gem.gem_type.color(),
                            );
                            draw_text("D", x + 18.0, gem_y + 40.0, 30.0, WHITE);
                        }
                        GemType::Barrel => {
                            draw_circle(
                                x + GEM_SIZE / 2.0,
                                gem_y + GEM_SIZE / 2.0,
                                GEM_SIZE / 2.5,
                                gem.gem_type.color(),
                            );
                            draw_text("B", x + 18.0, gem_y + 40.0, 30.0, WHITE);
                        }
                        GemType::Mixer => {
                            for i in 0..6 {
                                let angle = (i as f32 / 6.0) * std::f32::consts::PI * 2.0;
                                let r = GEM_SIZE / 3.0;
                                let px = x + GEM_SIZE / 2.0 + angle.cos() * r;
                                let py = gem_y + GEM_SIZE / 2.0 + angle.sin() * r;
                                draw_circle(px, py, 8.0, gem.gem_type.color());
                            }
                        }
                        GemType::Garbage => {
                            draw_rectangle(
                                x + 5.0, gem_y + 5.0,
                                GEM_SIZE - 10.0, GEM_SIZE - 10.0,
                                gem.gem_type.color(),
                            );
                            draw_text("X", x + 18.0, gem_y + 40.0, 30.0, RED);
                        }
                    }
                }

                // Selection highlight
                if let Some((sel_row, sel_col)) = self.selected {
                    if sel_row == row && sel_col == col {
                        draw_rectangle_lines(x, y, GEM_SIZE, GEM_SIZE, 4.0, YELLOW);
                    }
                }
            }
        }
    }

    fn draw_game_over(&self) {
        self.draw_game();

        draw_rectangle(
            0.0, 0.0, screen_width(), screen_height(),
            Color::from_rgba(0, 0, 0, 200),
        );

        let screen_width = screen_width();
        let screen_height = screen_height();

        draw_text(
            "GAME OVER",
            screen_width / 2.0 - 150.0,
            screen_height / 2.0 - 120.0,
            60.0,
            WHITE,
        );

        // Check for disconnect
        if let Some(reason) = &self.disconnect_reason {
            draw_text(
                reason,
                screen_width / 2.0 - 200.0,
                screen_height / 2.0 - 50.0,
                40.0,
                RED,
            );
        } else {
            let result_text = if self.score > self.opponent_score {
                "YOU WIN!"
            } else if self.score < self.opponent_score {
                "YOU LOSE!"
            } else {
                "IT'S A TIE!"
            };

            let result_color = if self.score > self.opponent_score {
                GREEN
            } else if self.score < self.opponent_score {
                RED
            } else {
                YELLOW
            };

            draw_text(
                result_text,
                screen_width / 2.0 - 100.0,
                screen_height / 2.0 - 50.0,
                50.0,
                result_color,
            );
        }

        draw_text(
            &format!("Your Score: {}", self.score),
            screen_width / 2.0 - 120.0,
            screen_height / 2.0 + 20.0,
            35.0,
            YELLOW,
        );

        draw_text(
            &format!("Opponent: {}", self.opponent_score),
            screen_width / 2.0 - 120.0,
            screen_height / 2.0 + 60.0,
            35.0,
            Color::from_rgba(255, 100, 100, 255),
        );

        // Rematch status (for online mode)
        if self.network_mode == NetworkMode::Online {
            if self.requested_rematch && self.opponent_requested_rematch {
                draw_text(
                    "Starting rematch...",
                    screen_width / 2.0 - 120.0,
                    screen_height / 2.0 + 95.0,
                    25.0,
                    GREEN,
                );
            } else if self.requested_rematch {
                draw_text(
                    "Waiting for opponent...",
                    screen_width / 2.0 - 140.0,
                    screen_height / 2.0 + 95.0,
                    25.0,
                    YELLOW,
                );
            } else if self.opponent_requested_rematch {
                draw_text(
                    "Opponent wants rematch!",
                    screen_width / 2.0 - 140.0,
                    screen_height / 2.0 + 95.0,
                    25.0,
                    ORANGE,
                );
            }
        }

        let button_x = screen_width / 2.0 - 100.0;
        let button_y = screen_height / 2.0 + 120.0;

        let button_color = if self.requested_rematch {
            Color::from_rgba(100, 100, 100, 255) // Gray if already requested
        } else {
            GREEN
        };

        let button_text = if self.network_mode == NetworkMode::Online {
            if self.requested_rematch {
                "REMATCH REQUESTED"
            } else {
                "REQUEST REMATCH"
            }
        } else {
            "PLAY AGAIN"
        };

        draw_rectangle(button_x, button_y, 200.0, 50.0, button_color);
        draw_text(
            button_text,
            button_x + if self.network_mode == NetworkMode::Online && !self.requested_rematch { 15.0 } else { 10.0 },
            button_y + 33.0,
            if button_text.len() > 15 { 20.0 } else { 30.0 },
            WHITE
        );
    }

    fn draw_leaderboard(&self) {
        let screen_width = screen_width();
        let screen_height = screen_height();

        draw_text(
            "LEADERBOARD - TOP 10",
            screen_width / 2.0 - 180.0,
            80.0,
            40.0,
            WHITE,
        );

        // Display leaderboard data
        let start_y = 150.0;
        let row_height = 50.0;

        if self.leaderboard_data.is_empty() {
            draw_text(
                "No leaderboard data available.",
                screen_width / 2.0 - 150.0,
                start_y,
                25.0,
                LIGHTGRAY,
            );
            draw_text(
                "Connect to online mode to view rankings.",
                screen_width / 2.0 - 200.0,
                start_y + 40.0,
                20.0,
                GRAY,
            );
        } else {
            for (i, (username, elo)) in self.leaderboard_data.iter().enumerate() {
                let y = start_y + i as f32 * row_height;

                // Background for each row
                let bg_color = if i % 2 == 0 {
                    Color::from_rgba(40, 40, 70, 255)
                } else {
                    Color::from_rgba(30, 30, 60, 255)
                };
                draw_rectangle(
                    screen_width / 2.0 - 250.0,
                    y - 35.0,
                    500.0,
                    45.0,
                    bg_color
                );

                // Rank
                let rank_color = match i {
                    0 => Color::from_rgba(255, 215, 0, 255),  // Gold
                    1 => Color::from_rgba(192, 192, 192, 255), // Silver
                    2 => Color::from_rgba(205, 127, 50, 255),  // Bronze
                    _ => WHITE,
                };
                draw_text(
                    &format!("#{}", i + 1),
                    screen_width / 2.0 - 230.0,
                    y,
                    30.0,
                    rank_color,
                );

                // Username
                draw_text(
                    username,
                    screen_width / 2.0 - 150.0,
                    y,
                    28.0,
                    WHITE,
                );

                // ELO
                draw_text(
                    &format!("{} ELO", elo),
                    screen_width / 2.0 + 100.0,
                    y,
                    28.0,
                    YELLOW,
                );
            }
        }

        // Back button
        let back_x = screen_width / 2.0 - 100.0;
        let back_y = screen_height - 100.0;
        draw_rectangle(back_x, back_y, 200.0, 50.0, Color::from_rgba(100, 100, 100, 255));
        draw_text("BACK TO MENU", back_x + 25.0, back_y + 33.0, 25.0, WHITE);
    }
}

#[derive(Clone)]
enum MatchType {
    Line(Vec<(usize, usize)>),
    LShape(Vec<(usize, usize)>),
    TShape(Vec<(usize, usize)>),
}

impl MatchType {
    fn positions(&self) -> &Vec<(usize, usize)> {
        match self {
            MatchType::Line(p) | MatchType::LShape(p) | MatchType::TShape(p) => p,
        }
    }
}

// Async function to connect to WebSocket server
async fn connect_to_server() -> Option<NetworkBridge> {
    let url = "ws://127.0.0.1:9001";

    println!("Attempting to connect to server at {}...", url);

    match connect_async(url).await {
        Ok((ws_stream, _)) => {
            println!("Connected to server!");

            let (write, read) = ws_stream.split();

            // Create channels
            let (to_server_tx, mut to_server_rx) = unbounded_channel::<ClientMessage>();
            let (from_server_tx, from_server_rx) = unbounded_channel::<ServerMessage>();

            // Spawn task to handle WebSocket communication
            tokio::spawn(async move {
                let mut write = write;
                let mut read = read;

                loop {
                    tokio::select! {
                        // Receive from game and send to server
                        Some(msg) = to_server_rx.recv() => {
                            let json = serde_json::to_string(&msg).unwrap();
                            if write.send(Message::Text(json)).await.is_err() {
                                println!("Failed to send message to server");
                                break;
                            }
                        }

                        // Receive from server and send to game
                        result = read.next() => {
                            match result {
                                Some(Ok(Message::Text(text))) => {
                                    if let Ok(msg) = serde_json::from_str::<ServerMessage>(&text) {
                                        if from_server_tx.send(msg).is_err() {
                                            println!("Failed to send message to game");
                                            break;
                                        }
                                    }
                                }
                                Some(Ok(Message::Close(_))) | None => {
                                    println!("Server disconnected");
                                    break;
                                }
                                Some(Err(e)) => {
                                    println!("WebSocket error: {}", e);
                                    break;
                                }
                                _ => {}
                            }
                        }
                    }
                }
            });

            Some(NetworkBridge {
                to_server: to_server_tx,
                from_server: from_server_rx,
            })
        }
        Err(e) => {
            println!("Failed to connect to server: {}", e);
            None
        }
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Brick City Wars - Match3 PVP".to_owned(),
        window_width: 600,
        window_height: 850,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut game = Game::new();
    let mut connecting = false;

    loop {
        let dt = get_frame_time();

        // Handle connection attempt
        if game.state == GameState::Connecting && !connecting {
            connecting = true;
            // Attempt to connect to server
            if let Some(bridge) = connect_to_server().await {
                game.set_network_bridge(bridge);
                // Send Login message immediately after connecting
                if let Some(network_bridge) = &game.network_bridge {
                    network_bridge.send(ClientMessage::Login {
                        username: game.username.clone(),
                    });
                }
                // Wait for AuthAccepted/AuthRejected in handle_server_message
            } else {
                println!("Failed to connect - falling back to offline mode");
                game.network_mode = NetworkMode::Offline;
                game.state = GameState::Login;
                game.disconnect_reason = Some("Connection failed".to_string());
            }
            connecting = false;
        }

        game.update(dt);

        if is_mouse_button_pressed(MouseButton::Left) {
            let (mouse_x, mouse_y) = mouse_position();

            match game.state {
                GameState::Menu => {
                    let screen_width = screen_width();
                    let screen_height = screen_height();

                    // Offline button
                    let offline_x = screen_width / 2.0 - 100.0;
                    let offline_y = screen_height / 2.0 - 20.0;
                    if mouse_x >= offline_x && mouse_x <= offline_x + 200.0
                        && mouse_y >= offline_y && mouse_y <= offline_y + 50.0 {
                        game.start_game(false);
                    }

                    // Online button
                    let online_x = screen_width / 2.0 - 100.0;
                    let online_y = screen_height / 2.0 + 50.0;
                    if mouse_x >= online_x && mouse_x <= online_x + 200.0
                        && mouse_y >= online_y && mouse_y <= online_y + 50.0 {
                        game.state = GameState::Login;
                        game.pending_username.clear();
                        game.disconnect_reason = None;
                    }

                    // Leaderboard button
                    let leaderboard_x = screen_width / 2.0 - 100.0;
                    let leaderboard_y = screen_height / 2.0 + 120.0;
                    if mouse_x >= leaderboard_x && mouse_x <= leaderboard_x + 200.0
                        && mouse_y >= leaderboard_y && mouse_y <= leaderboard_y + 50.0 {
                        game.connecting_for_leaderboard = true;
                        game.state = GameState::Login;
                        game.pending_username.clear();
                        game.disconnect_reason = None;
                    }
                }
                GameState::Leaderboard => {
                    let screen_width = screen_width();
                    let screen_height = screen_height();

                    // Back button
                    let back_x = screen_width / 2.0 - 100.0;
                    let back_y = screen_height - 100.0;
                    if mouse_x >= back_x && mouse_x <= back_x + 200.0
                        && mouse_y >= back_y && mouse_y <= back_y + 50.0 {
                        game.state = GameState::Menu;
                    }
                }
                GameState::Login => {
                    let screen_width = screen_width();
                    let screen_height = screen_height();

                    // Continue button
                    let button_x = screen_width / 2.0 - 100.0;
                    let button_y = screen_height / 2.0 + 50.0;
                    if !game.pending_username.is_empty()
                        && mouse_x >= button_x && mouse_x <= button_x + 200.0
                        && mouse_y >= button_y && mouse_y <= button_y + 50.0 {
                        // Submit username and transition to Connecting
                        game.username = game.pending_username.clone();
                        game.state = GameState::Connecting;
                        game.network_mode = NetworkMode::Online;
                    }

                    // Back button
                    let back_x = screen_width / 2.0 - 100.0;
                    let back_y = screen_width / 2.0 + 170.0;
                    if mouse_x >= back_x && mouse_x <= back_x + 200.0
                        && mouse_y >= back_y && mouse_y <= back_y + 40.0 {
                        game.state = GameState::Menu;
                        game.pending_username.clear();
                    }
                }
                GameState::Connecting => {
                    // Waiting for connection - handled by async task
                }
                GameState::Playing => {
                    game.handle_click(mouse_x, mouse_y);
                }
                GameState::GameOver => {
                    let screen_width = screen_width();
                    let screen_height = screen_height();
                    let button_x = screen_width / 2.0 - 100.0;
                    let button_y = screen_height / 2.0 + 120.0;

                    if mouse_x >= button_x && mouse_x <= button_x + 200.0
                        && mouse_y >= button_y && mouse_y <= button_y + 50.0 {
                        if game.network_mode == NetworkMode::Online {
                            // Request rematch in online mode
                            if !game.requested_rematch {
                                game.requested_rematch = true;
                                if let Some(bridge) = &game.network_bridge {
                                    bridge.send(ClientMessage::RequestRematch);
                                }
                            }
                        } else {
                            // Go back to menu in offline mode
                            game.state = GameState::Menu;
                        }
                    }
                }
                _ => {}
            }
        }

        // Handle keyboard input for login screen
        if game.state == GameState::Login {
            // Get character input
            if let Some(character) = get_char_pressed() {
                if character.is_alphanumeric() || character == '_' || character == '-' {
                    if game.pending_username.len() < 20 {
                        game.pending_username.push(character);
                    }
                }
            }

            // Handle backspace
            if is_key_pressed(KeyCode::Backspace) {
                game.pending_username.pop();
            }

            // Handle enter key to submit
            if is_key_pressed(KeyCode::Enter) && !game.pending_username.is_empty() {
                game.username = game.pending_username.clone();
                game.state = GameState::Connecting;
                game.network_mode = NetworkMode::Online;
            }

            // Handle escape to go back
            if is_key_pressed(KeyCode::Escape) {
                game.state = GameState::Menu;
                game.pending_username.clear();
            }
        }

        // Handle keyboard input for boosters (keys 1, 2, 3)
        if game.state == GameState::Playing {
            if is_key_pressed(KeyCode::Key1) {
                game.activate_booster(0);
            }
            if is_key_pressed(KeyCode::Key2) {
                game.activate_booster(1);
            }
            if is_key_pressed(KeyCode::Key3) {
                game.activate_booster(2);
            }
        }

        game.draw();
        next_frame().await;
    }
}

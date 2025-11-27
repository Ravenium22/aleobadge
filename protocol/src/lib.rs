use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type PlayerId = Uuid;
pub type GameId = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    Login { username: String },
    JoinQueue,
    SwapGems { row1: usize, col1: usize, row2: usize, col2: usize },
    ScoreUpdate { score: u32 },
    SendGarbage { amount: u8 },
    ActivateSpecial { row: usize, col: usize },
    ActivateBooster { booster_id: u8 },
    RequestRematch,
    LeaveGame,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    AuthAccepted { player_id: PlayerId, username: String, elo: i32, wins: u32, losses: u32 },
    AuthRejected { reason: String },
    Connected { player_id: PlayerId },
    Queued { position: usize },
    MatchFound { game_id: GameId, opponent_id: PlayerId },
    GameStarted { game_id: GameId },
    OpponentSwap { row1: usize, col1: usize, row2: usize, col2: usize },
    ScoreUpdate { player_score: u32, opponent_score: u32 },
    TimeUpdate { seconds_remaining: u64 },
    ReceiveGarbage { amount: u8 },
    OpponentActivatedSpecial { row: usize, col: usize },
    OpponentActivatedBooster { booster_id: u8 },
    GameOver { winner: GameResult },
    MatchResult { new_elo: i32, elo_change: i32, wins: u32, losses: u32 },
    OpponentRequestedRematch,
    RematchAccepted,
    OpponentLeft,
    OpponentDisconnected,
    Error { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GameResult {
    Win,
    Loss,
    Tie,
}

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type PlayerId = Uuid;
pub type GameId = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    JoinQueue,
    SwapGems { row1: usize, col1: usize, row2: usize, col2: usize },
    ScoreUpdate { score: u32 },
    LeaveGame,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    Connected { player_id: PlayerId },
    Queued { position: usize },
    MatchFound { game_id: GameId, opponent_id: PlayerId },
    GameStarted { game_id: GameId },
    OpponentSwap { row1: usize, col1: usize, row2: usize, col2: usize },
    ScoreUpdate { player_score: u32, opponent_score: u32 },
    TimeUpdate { seconds_remaining: u64 },
    GameOver { winner: GameResult },
    OpponentDisconnected,
    Error { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GameResult {
    Win,
    Loss,
    Tie,
}

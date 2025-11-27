use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock, Mutex};
use tokio::time::{interval, Duration};
use futures::{StreamExt, SinkExt};
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;
use match3_protocol::{ClientMessage, ServerMessage, GameResult, PlayerId, GameId};

type Tx = mpsc::UnboundedSender<Message>;

const GAME_DURATION: u64 = 90; // seconds

// Player representation
#[derive(Debug, Clone)]
struct Player {
    id: PlayerId,
    tx: Tx,
}

// Game Session
#[derive(Debug)]
struct GameSession {
    id: GameId,
    player1: Player,
    player2: Player,
    scores: Arc<RwLock<(u32, u32)>>, // (player1_score, player2_score)
    start_time: std::time::Instant,
    active: Arc<RwLock<bool>>,
}

impl GameSession {
    fn new(player1: Player, player2: Player) -> Self {
        let game_id = Uuid::new_v4();

        Self {
            id: game_id,
            player1,
            player2,
            scores: Arc::new(RwLock::new((0, 0))),
            start_time: std::time::Instant::now(),
            active: Arc::new(RwLock::new(true)),
        }
    }

    async fn start(&self) {
        // Notify both players that the game has started
        let start_msg = ServerMessage::GameStarted { game_id: self.id };
        let _ = self.player1.tx.send(Message::Text(
            serde_json::to_string(&start_msg).unwrap()
        ));
        let _ = self.player2.tx.send(Message::Text(
            serde_json::to_string(&start_msg).unwrap()
        ));

        // Start the game timer
        self.run_game_timer().await;
    }

    async fn run_game_timer(&self) {
        let mut ticker = interval(Duration::from_secs(1));
        let scores = Arc::clone(&self.scores);
        let active = Arc::clone(&self.active);
        let p1_tx = self.player1.tx.clone();
        let p2_tx = self.player2.tx.clone();
        let p1_id = self.player1.id;
        let p2_id = self.player2.id;

        tokio::spawn(async move {
            for i in 0..GAME_DURATION {
                ticker.tick().await;

                let is_active = *active.read().await;
                if !is_active {
                    break;
                }

                let remaining = GAME_DURATION - i - 1;
                let time_msg = ServerMessage::TimeUpdate {
                    seconds_remaining: remaining
                };
                let time_str = serde_json::to_string(&time_msg).unwrap();

                let _ = p1_tx.send(Message::Text(time_str.clone()));
                let _ = p2_tx.send(Message::Text(time_str));
            }

            // Game ended - determine winner
            let (score1, score2) = *scores.read().await;

            let p1_result = if score1 > score2 {
                GameResult::Win
            } else if score1 < score2 {
                GameResult::Loss
            } else {
                GameResult::Tie
            };

            let p2_result = if score2 > score1 {
                GameResult::Win
            } else if score2 < score1 {
                GameResult::Loss
            } else {
                GameResult::Tie
            };

            let p1_msg = ServerMessage::GameOver { winner: p1_result };
            let p2_msg = ServerMessage::GameOver { winner: p2_result };

            let _ = p1_tx.send(Message::Text(serde_json::to_string(&p1_msg).unwrap()));
            let _ = p2_tx.send(Message::Text(serde_json::to_string(&p2_msg).unwrap()));

            *active.write().await = false;
        });
    }

    async fn handle_swap(&self, from_player: PlayerId, row1: usize, col1: usize, row2: usize, col2: usize) {
        // Notify the opponent about the swap
        let swap_msg = ServerMessage::OpponentSwap { row1, col1, row2, col2 };
        let swap_str = serde_json::to_string(&swap_msg).unwrap();

        if from_player == self.player1.id {
            let _ = self.player2.tx.send(Message::Text(swap_str));
        } else {
            let _ = self.player1.tx.send(Message::Text(swap_str));
        }
    }

    async fn update_score(&self, player_id: PlayerId, new_score: u32) {
        let mut scores = self.scores.write().await;

        if player_id == self.player1.id {
            scores.0 = new_score;

            // Send score update to both players
            let msg1 = ServerMessage::ScoreUpdate {
                player_score: scores.0,
                opponent_score: scores.1
            };
            let msg2 = ServerMessage::ScoreUpdate {
                player_score: scores.1,
                opponent_score: scores.0
            };

            let _ = self.player1.tx.send(Message::Text(serde_json::to_string(&msg1).unwrap()));
            let _ = self.player2.tx.send(Message::Text(serde_json::to_string(&msg2).unwrap()));
        } else {
            scores.1 = new_score;

            let msg1 = ServerMessage::ScoreUpdate {
                player_score: scores.0,
                opponent_score: scores.1
            };
            let msg2 = ServerMessage::ScoreUpdate {
                player_score: scores.1,
                opponent_score: scores.0
            };

            let _ = self.player1.tx.send(Message::Text(serde_json::to_string(&msg1).unwrap()));
            let _ = self.player2.tx.send(Message::Text(serde_json::to_string(&msg2).unwrap()));
        }
    }

    async fn handle_disconnect(&self, player_id: PlayerId) {
        *self.active.write().await = false;

        let disconnect_msg = ServerMessage::OpponentDisconnected;
        let disconnect_str = serde_json::to_string(&disconnect_msg).unwrap();

        // Notify the other player
        if player_id == self.player1.id {
            let _ = self.player2.tx.send(Message::Text(disconnect_str));
        } else {
            let _ = self.player1.tx.send(Message::Text(disconnect_str));
        }
    }
}

// Server State
#[derive(Clone)]
struct ServerState {
    players: Arc<RwLock<HashMap<PlayerId, Player>>>,
    games: Arc<RwLock<HashMap<GameId, Arc<GameSession>>>>,
    matchmaking_queue: Arc<Mutex<Vec<PlayerId>>>,
    player_to_game: Arc<RwLock<HashMap<PlayerId, GameId>>>,
}

impl ServerState {
    fn new() -> Self {
        Self {
            players: Arc::new(RwLock::new(HashMap::new())),
            games: Arc::new(RwLock::new(HashMap::new())),
            matchmaking_queue: Arc::new(Mutex::new(Vec::new())),
            player_to_game: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn add_player(&self, player: Player) {
        let player_id = player.id;
        self.players.write().await.insert(player_id, player.clone());

        // Send connection confirmation
        let msg = ServerMessage::Connected { player_id };
        let _ = player.tx.send(Message::Text(serde_json::to_string(&msg).unwrap()));
    }

    async fn remove_player(&self, player_id: PlayerId) {
        self.players.write().await.remove(&player_id);

        // Remove from queue if present
        let mut queue = self.matchmaking_queue.lock().await;
        queue.retain(|&id| id != player_id);
        drop(queue);

        // Handle game disconnect
        if let Some(game_id) = self.player_to_game.read().await.get(&player_id) {
            if let Some(game) = self.games.read().await.get(game_id) {
                game.handle_disconnect(player_id).await;
            }
            self.player_to_game.write().await.remove(&player_id);
        }
    }

    async fn join_queue(&self, player_id: PlayerId) {
        let mut queue = self.matchmaking_queue.lock().await;

        if !queue.contains(&player_id) {
            queue.push(player_id);

            let position = queue.len();
            if let Some(player) = self.players.read().await.get(&player_id) {
                let msg = ServerMessage::Queued { position };
                let _ = player.tx.send(Message::Text(serde_json::to_string(&msg).unwrap()));
            }

            // Try to match players
            if queue.len() >= 2 {
                let p1_id = queue.remove(0);
                let p2_id = queue.remove(0);
                drop(queue);

                self.create_match(p1_id, p2_id).await;
            }
        }
    }

    async fn create_match(&self, p1_id: PlayerId, p2_id: PlayerId) {
        let players = self.players.read().await;

        if let (Some(p1), Some(p2)) = (players.get(&p1_id), players.get(&p2_id)) {
            let game = Arc::new(GameSession::new(p1.clone(), p2.clone()));
            let game_id = game.id;

            // Notify both players of the match
            let match_msg_p1 = ServerMessage::MatchFound {
                game_id,
                opponent_id: p2_id
            };
            let match_msg_p2 = ServerMessage::MatchFound {
                game_id,
                opponent_id: p1_id
            };

            let _ = p1.tx.send(Message::Text(serde_json::to_string(&match_msg_p1).unwrap()));
            let _ = p2.tx.send(Message::Text(serde_json::to_string(&match_msg_p2).unwrap()));

            // Register game
            self.games.write().await.insert(game_id, game.clone());
            self.player_to_game.write().await.insert(p1_id, game_id);
            self.player_to_game.write().await.insert(p2_id, game_id);

            // Start the game
            game.start().await;
        }
    }

    async fn handle_client_message(&self, player_id: PlayerId, msg: ClientMessage) {
        match msg {
            ClientMessage::JoinQueue => {
                self.join_queue(player_id).await;
            }
            ClientMessage::SwapGems { row1, col1, row2, col2 } => {
                if let Some(game_id) = self.player_to_game.read().await.get(&player_id) {
                    if let Some(game) = self.games.read().await.get(game_id) {
                        game.handle_swap(player_id, row1, col1, row2, col2).await;
                    }
                }
            }
            ClientMessage::ScoreUpdate { score } => {
                if let Some(game_id) = self.player_to_game.read().await.get(&player_id) {
                    if let Some(game) = self.games.read().await.get(game_id) {
                        game.update_score(player_id, score).await;
                    }
                }
            }
            ClientMessage::LeaveGame => {
                self.remove_player(player_id).await;
            }
        }
    }
}

async fn handle_connection(
    ws_stream: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    state: ServerState,
) {
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let (tx, mut rx) = mpsc::unbounded_channel();

    let player_id = Uuid::new_v4();
    let player = Player {
        id: player_id,
        tx: tx.clone(),
    };

    // Add player to server state
    state.add_player(player).await;

    println!("New player connected: {}", player_id);

    // Spawn task to send messages to client
    let mut send_task = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            if ws_sender.send(message).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming messages from client
    let state_clone = state.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(message)) = ws_receiver.next().await {
            if let Message::Text(text) = message {
                if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                    state_clone.handle_client_message(player_id, client_msg).await;
                }
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = &mut send_task => {
            recv_task.abort();
        }
        _ = &mut recv_task => {
            send_task.abort();
        }
    }

    // Clean up
    state.remove_player(player_id).await;
    println!("Player disconnected: {}", player_id);
}

#[tokio::main]
async fn main() {
    let addr = "127.0.0.1:9001";
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("Match3 PVP Server listening on: {}", addr);

    let state = ServerState::new();

    while let Ok((stream, addr)) = listener.accept().await {
        println!("New connection from: {}", addr);

        let state_clone = state.clone();
        tokio::spawn(async move {
            match tokio_tungstenite::accept_async(stream).await {
                Ok(ws_stream) => {
                    handle_connection(ws_stream, state_clone).await;
                }
                Err(e) => {
                    eprintln!("WebSocket handshake error: {}", e);
                }
            }
        });
    }
}

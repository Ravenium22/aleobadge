use sqlx::{SqlitePool, Row};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub elo: i32,
    pub wins: u32,
    pub losses: u32,
    pub bricks: u32,
    pub gold: u32,
}

#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// Initialize database connection and run migrations
    pub async fn init() -> Result<Self, sqlx::Error> {
        // Create database file if it doesn't exist
        let pool = SqlitePool::connect("sqlite:match3.db").await?;

        // Run migrations - create users table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                username TEXT UNIQUE NOT NULL,
                elo INTEGER DEFAULT 1000,
                wins INTEGER DEFAULT 0,
                losses INTEGER DEFAULT 0,
                bricks INTEGER DEFAULT 0,
                gold INTEGER DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await?;

        println!("Database initialized successfully");
        Ok(Database { pool })
    }

    /// Get user by username, or create if doesn't exist
    pub async fn get_or_create_user(&self, username: &str) -> Result<User, sqlx::Error> {
        // Try to get existing user first
        let existing = sqlx::query(
            "SELECT id, username, elo, wins, losses, bricks, gold FROM users WHERE username = ?"
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = existing {
            let id_str: String = row.get("id");
            let id = Uuid::parse_str(&id_str).unwrap();

            return Ok(User {
                id,
                username: row.get("username"),
                elo: row.get("elo"),
                wins: row.get("wins"),
                losses: row.get("losses"),
                bricks: row.get("bricks"),
                gold: row.get("gold"),
            });
        }

        // User doesn't exist - create new one
        let new_id = Uuid::new_v4();
        let id_str = new_id.to_string();

        sqlx::query(
            "INSERT INTO users (id, username, elo, wins, losses, bricks, gold) VALUES (?, ?, 1000, 0, 0, 0, 0)"
        )
        .bind(&id_str)
        .bind(username)
        .execute(&self.pool)
        .await?;

        println!("Created new user: {} ({})", username, new_id);

        Ok(User {
            id: new_id,
            username: username.to_string(),
            elo: 1000,
            wins: 0,
            losses: 0,
            bricks: 0,
            gold: 0,
        })
    }

    /// Update match result with ELO calculation and resource rewards
    pub async fn update_match_result(
        &self,
        winner_id: Uuid,
        loser_id: Uuid,
        is_tie: bool,
    ) -> Result<(User, User), sqlx::Error> {
        // Get current stats for both players
        let winner = self.get_user_by_id(winner_id).await?;
        let loser = self.get_user_by_id(loser_id).await?;

        // Calculate ELO changes using standard formula
        let (winner_new_elo, loser_new_elo) = if is_tie {
            // Tie - smaller ELO change
            calculate_elo_change(winner.elo, loser.elo, 0.5)
        } else {
            // Winner gets full points
            calculate_elo_change(winner.elo, loser.elo, 1.0)
        };

        // Calculate resource rewards
        let (winner_bricks, winner_gold, loser_bricks, loser_gold) = if is_tie {
            // Tie: +50 Bricks, +5 Gold for both
            (50, 5, 50, 5)
        } else {
            // Winner: +100 Bricks, +10 Gold | Loser: +25 Bricks
            (100, 10, 25, 0)
        };

        // Start transaction
        let mut tx = self.pool.begin().await?;

        // Update winner
        sqlx::query(
            "UPDATE users SET elo = ?, wins = wins + ?, bricks = bricks + ?, gold = gold + ? WHERE id = ?"
        )
        .bind(winner_new_elo)
        .bind(if is_tie { 0 } else { 1 })
        .bind(winner_bricks)
        .bind(winner_gold)
        .bind(winner_id.to_string())
        .execute(&mut *tx)
        .await?;

        // Update loser
        sqlx::query(
            "UPDATE users SET elo = ?, losses = losses + ?, bricks = bricks + ?, gold = gold + ? WHERE id = ?"
        )
        .bind(loser_new_elo)
        .bind(if is_tie { 0 } else { 1 })
        .bind(loser_bricks)
        .bind(loser_gold)
        .bind(loser_id.to_string())
        .execute(&mut *tx)
        .await?;

        // Commit transaction
        tx.commit().await?;

        // Fetch updated stats
        let winner_updated = self.get_user_by_id(winner_id).await?;
        let loser_updated = self.get_user_by_id(loser_id).await?;

        Ok((winner_updated, loser_updated))
    }

    /// Get user by ID
    async fn get_user_by_id(&self, id: Uuid) -> Result<User, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id, username, elo, wins, losses, bricks, gold FROM users WHERE id = ?"
        )
        .bind(id.to_string())
        .fetch_one(&self.pool)
        .await?;

        let id_str: String = row.get("id");
        let parsed_id = Uuid::parse_str(&id_str).unwrap();

        Ok(User {
            id: parsed_id,
            username: row.get("username"),
            elo: row.get("elo"),
            wins: row.get("wins"),
            losses: row.get("losses"),
            bricks: row.get("bricks"),
            gold: row.get("gold"),
        })
    }

    /// Get top 10 players by ELO for leaderboard
    pub async fn get_leaderboard(&self) -> Result<Vec<(String, i32)>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT username, elo FROM users ORDER BY elo DESC LIMIT 10"
        )
        .fetch_all(&self.pool)
        .await?;

        let leaderboard = rows.iter().map(|row| {
            let username: String = row.get("username");
            let elo: i32 = row.get("elo");
            (username, elo)
        }).collect();

        Ok(leaderboard)
    }
}

/// Calculate ELO change using standard formula
/// K-factor = 32 (standard for chess)
fn calculate_elo_change(winner_elo: i32, loser_elo: i32, score: f64) -> (i32, i32) {
    const K: f64 = 32.0;

    // Expected scores
    let winner_expected = 1.0 / (1.0 + 10_f64.powf((loser_elo - winner_elo) as f64 / 400.0));
    let loser_expected = 1.0 - winner_expected;

    // Calculate changes
    let winner_change = (K * (score - winner_expected)).round() as i32;
    let loser_change = (K * ((1.0 - score) - loser_expected)).round() as i32;

    (winner_elo + winner_change, loser_elo + loser_change)
}

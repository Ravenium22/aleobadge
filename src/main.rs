use macroquad::prelude::*;
use ::rand::Rng;

const GRID_SIZE: usize = 8;
const GEM_SIZE: f32 = 60.0;
const BOARD_OFFSET_X: f32 = 50.0;
const BOARD_OFFSET_Y: f32 = 120.0;
const GAME_DURATION: f32 = 90.0;

#[derive(Clone, Copy, Debug, PartialEq)]
enum GemType {
    Red,
    Blue,
    Green,
    Yellow,
    Purple,
    Orange,
}

impl GemType {
    fn random() -> Self {
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

    fn color(&self) -> Color {
        match self {
            GemType::Red => Color::from_rgba(255, 50, 50, 255),
            GemType::Blue => Color::from_rgba(50, 100, 255, 255),
            GemType::Green => Color::from_rgba(50, 255, 100, 255),
            GemType::Yellow => Color::from_rgba(255, 255, 50, 255),
            GemType::Purple => Color::from_rgba(200, 50, 255, 255),
            GemType::Orange => Color::from_rgba(255, 150, 50, 255),
        }
    }
}

#[derive(Clone, Copy)]
struct Gem {
    gem_type: GemType,
    y_offset: f32,
    is_falling: bool,
}

impl Gem {
    fn new(gem_type: GemType) -> Self {
        Self {
            gem_type,
            y_offset: 0.0,
            is_falling: false,
        }
    }
}

#[derive(PartialEq)]
enum GameState {
    Menu,
    Playing,
    GameOver,
}

struct Game {
    grid: Vec<Vec<Option<Gem>>>,
    selected: Option<(usize, usize)>,
    score: u32,
    opponent_score: u32,
    state: GameState,
    time_remaining: f32,
    animation_timer: f32,
}

impl Game {
    fn new() -> Self {
        let mut game = Self {
            grid: vec![vec![None; GRID_SIZE]; GRID_SIZE],
            selected: None,
            score: 0,
            opponent_score: 0,
            state: GameState::Menu,
            time_remaining: GAME_DURATION,
            animation_timer: 0.0,
        };
        game.initialize_board();
        game
    }

    fn initialize_board(&mut self) {
        // Fill board with random gems, avoiding initial matches
        for row in 0..GRID_SIZE {
            for col in 0..GRID_SIZE {
                loop {
                    let gem = Gem::new(GemType::random());
                    self.grid[row][col] = Some(gem);

                    // Check if this creates a match-3
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

    fn start_game(&mut self) {
        self.state = GameState::Playing;
        self.score = 0;
        self.opponent_score = 0;
        self.time_remaining = GAME_DURATION;
        self.selected = None;
        self.initialize_board();
    }

    fn update(&mut self, dt: f32) {
        match self.state {
            GameState::Playing => {
                self.time_remaining -= dt;
                if self.time_remaining <= 0.0 {
                    self.time_remaining = 0.0;
                    self.state = GameState::GameOver;
                }

                // Update animations
                if self.animation_timer > 0.0 {
                    self.animation_timer -= dt;
                } else {
                    // Process falling gems
                    self.update_falling_gems(dt);
                }

                // Simulate opponent score increasing
                if ::rand::random::<f32>() < 0.01 {
                    self.opponent_score += ::rand::thread_rng().gen_range(10..50);
                }
            }
            _ => {}
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

        if let Some((sel_row, sel_col)) = self.selected {
            // Check if clicked gem is adjacent
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
        let temp = self.grid[row1][col1];
        self.grid[row1][col1] = self.grid[row2][col2];
        self.grid[row2][col2] = temp;

        // Check if swap creates matches
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

        let gem_type = self.grid[row][col].unwrap().gem_type;

        // Check horizontal
        let mut h_count = 1;
        // Count left
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
        // Count right
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
        // Count up
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
        // Count down
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
        let mut to_remove = vec![vec![false; GRID_SIZE]; GRID_SIZE];
        let mut total_matches = 0;

        // Find all matches
        for row in 0..GRID_SIZE {
            for col in 0..GRID_SIZE {
                if self.grid[row][col].is_none() {
                    continue;
                }

                let gem_type = self.grid[row][col].unwrap().gem_type;

                // Check horizontal matches
                let mut h_matches = vec![(row, col)];
                for c in (col + 1)..GRID_SIZE {
                    if let Some(g) = self.grid[row][c] {
                        if g.gem_type == gem_type {
                            h_matches.push((row, c));
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                if h_matches.len() >= 3 {
                    for &(r, c) in &h_matches {
                        to_remove[r][c] = true;
                    }
                }

                // Check vertical matches
                let mut v_matches = vec![(row, col)];
                for r in (row + 1)..GRID_SIZE {
                    if let Some(g) = self.grid[r][col] {
                        if g.gem_type == gem_type {
                            v_matches.push((r, col));
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                if v_matches.len() >= 3 {
                    for &(r, c) in &v_matches {
                        to_remove[r][c] = true;
                    }
                }
            }
        }

        // Remove matched gems and count
        for row in 0..GRID_SIZE {
            for col in 0..GRID_SIZE {
                if to_remove[row][col] {
                    self.grid[row][col] = None;
                    total_matches += 1;
                }
            }
        }

        if total_matches > 0 {
            self.score += total_matches * 10;
            if total_matches >= 4 {
                self.score += 20; // Bonus for 4+ matches
            }
            self.apply_gravity();
        }
    }

    fn apply_gravity(&mut self) {
        for col in 0..GRID_SIZE {
            let mut write_row = GRID_SIZE;

            // Move existing gems down
            for row in (0..GRID_SIZE).rev() {
                if self.grid[row][col].is_some() {
                    write_row -= 1;
                    if write_row != row {
                        self.grid[write_row][col] = self.grid[row][col];
                        self.grid[row][col] = None;
                    }
                }
            }

            // Fill empty spaces at top with new gems
            for row in 0..write_row {
                let mut new_gem = Gem::new(GemType::random());
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
            GameState::Menu => {
                self.draw_menu();
            }
            GameState::Playing => {
                self.draw_game();
            }
            GameState::GameOver => {
                self.draw_game_over();
            }
        }
    }

    fn draw_menu(&self) {
        let screen_width = screen_width();
        let screen_height = screen_height();

        draw_text(
            "MATCH 3 PVP",
            screen_width / 2.0 - 150.0,
            screen_height / 2.0 - 100.0,
            60.0,
            WHITE,
        );

        draw_text(
            "Real-time Match-3 Battle",
            screen_width / 2.0 - 120.0,
            screen_height / 2.0 - 40.0,
            25.0,
            LIGHTGRAY,
        );

        draw_text(
            "Match 3 or more gems to score points!",
            screen_width / 2.0 - 180.0,
            screen_height / 2.0,
            20.0,
            LIGHTGRAY,
        );

        draw_text(
            "You have 90 seconds to beat your opponent!",
            screen_width / 2.0 - 200.0,
            screen_height / 2.0 + 30.0,
            20.0,
            LIGHTGRAY,
        );

        // Draw start button
        let button_x = screen_width / 2.0 - 100.0;
        let button_y = screen_height / 2.0 + 80.0;
        draw_rectangle(button_x, button_y, 200.0, 50.0, GREEN);
        draw_text("START GAME", button_x + 30.0, button_y + 33.0, 30.0, WHITE);
    }

    fn draw_game(&self) {
        // Draw header background
        draw_rectangle(0.0, 0.0, screen_width(), 100.0, Color::from_rgba(30, 30, 60, 255));

        // Draw timer
        let minutes = (self.time_remaining / 60.0) as u32;
        let seconds = (self.time_remaining % 60.0) as u32;
        let timer_text = format!("Time: {:02}:{:02}", minutes, seconds);
        let timer_color = if self.time_remaining < 20.0 { RED } else { WHITE };
        draw_text(&timer_text, 20.0, 40.0, 40.0, timer_color);

        // Draw scores
        draw_text(
            &format!("Your Score: {}", self.score),
            20.0,
            80.0,
            30.0,
            YELLOW,
        );

        draw_text(
            &format!("Opponent: {}", self.opponent_score),
            screen_width() - 250.0,
            40.0,
            30.0,
            Color::from_rgba(255, 100, 100, 255),
        );

        // Draw winning/losing indicator
        if self.score > self.opponent_score {
            draw_text("WINNING!", screen_width() - 250.0, 75.0, 25.0, GREEN);
        } else if self.score < self.opponent_score {
            draw_text("LOSING!", screen_width() - 250.0, 75.0, 25.0, RED);
        } else {
            draw_text("TIED!", screen_width() - 250.0, 75.0, 25.0, YELLOW);
        }

        // Draw grid background
        draw_rectangle(
            BOARD_OFFSET_X - 10.0,
            BOARD_OFFSET_Y - 10.0,
            GRID_SIZE as f32 * GEM_SIZE + 20.0,
            GRID_SIZE as f32 * GEM_SIZE + 20.0,
            Color::from_rgba(40, 40, 70, 255),
        );

        // Draw gems
        for row in 0..GRID_SIZE {
            for col in 0..GRID_SIZE {
                let x = BOARD_OFFSET_X + col as f32 * GEM_SIZE;
                let y = BOARD_OFFSET_Y + row as f32 * GEM_SIZE;

                // Draw cell background
                draw_rectangle(
                    x + 2.0,
                    y + 2.0,
                    GEM_SIZE - 4.0,
                    GEM_SIZE - 4.0,
                    Color::from_rgba(30, 30, 50, 255),
                );

                if let Some(gem) = self.grid[row][col] {
                    let gem_y = y + gem.y_offset;

                    // Draw gem
                    draw_circle(
                        x + GEM_SIZE / 2.0,
                        gem_y + GEM_SIZE / 2.0,
                        GEM_SIZE / 2.5,
                        gem.gem_type.color(),
                    );

                    // Draw gem highlight
                    draw_circle(
                        x + GEM_SIZE / 2.0 - 8.0,
                        gem_y + GEM_SIZE / 2.0 - 8.0,
                        GEM_SIZE / 8.0,
                        Color::from_rgba(255, 255, 255, 150),
                    );
                }

                // Highlight selected gem
                if let Some((sel_row, sel_col)) = self.selected {
                    if sel_row == row && sel_col == col {
                        draw_rectangle_lines(
                            x,
                            y,
                            GEM_SIZE,
                            GEM_SIZE,
                            4.0,
                            YELLOW,
                        );
                    }
                }
            }
        }
    }

    fn draw_game_over(&self) {
        // Draw the final board state
        self.draw_game();

        // Draw semi-transparent overlay
        draw_rectangle(
            0.0,
            0.0,
            screen_width(),
            screen_height(),
            Color::from_rgba(0, 0, 0, 200),
        );

        let screen_width = screen_width();
        let screen_height = screen_height();

        // Draw game over text
        draw_text(
            "GAME OVER",
            screen_width / 2.0 - 150.0,
            screen_height / 2.0 - 120.0,
            60.0,
            WHITE,
        );

        // Draw result
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

        // Draw final scores
        draw_text(
            &format!("Your Score: {}", self.score),
            screen_width / 2.0 - 120.0,
            screen_height / 2.0 + 20.0,
            35.0,
            YELLOW,
        );

        draw_text(
            &format!("Opponent Score: {}", self.opponent_score),
            screen_width / 2.0 - 150.0,
            screen_height / 2.0 + 60.0,
            35.0,
            Color::from_rgba(255, 100, 100, 255),
        );

        // Draw play again button
        let button_x = screen_width / 2.0 - 100.0;
        let button_y = screen_height / 2.0 + 120.0;
        draw_rectangle(button_x, button_y, 200.0, 50.0, GREEN);
        draw_text("PLAY AGAIN", button_x + 30.0, button_y + 33.0, 30.0, WHITE);
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Match 3 PVP".to_owned(),
        window_width: 600,
        window_height: 800,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut game = Game::new();

    loop {
        let dt = get_frame_time();
        game.update(dt);

        // Handle input
        if is_mouse_button_pressed(MouseButton::Left) {
            let (mouse_x, mouse_y) = mouse_position();

            match game.state {
                GameState::Menu => {
                    let screen_width = screen_width();
                    let screen_height = screen_height();
                    let button_x = screen_width / 2.0 - 100.0;
                    let button_y = screen_height / 2.0 + 80.0;

                    if mouse_x >= button_x
                        && mouse_x <= button_x + 200.0
                        && mouse_y >= button_y
                        && mouse_y <= button_y + 50.0
                    {
                        game.start_game();
                    }
                }
                GameState::Playing => {
                    game.handle_click(mouse_x, mouse_y);
                }
                GameState::GameOver => {
                    let screen_width = screen_width();
                    let screen_height = screen_height();
                    let button_x = screen_width / 2.0 - 100.0;
                    let button_y = screen_height / 2.0 + 120.0;

                    if mouse_x >= button_x
                        && mouse_x <= button_x + 200.0
                        && mouse_y >= button_y
                        && mouse_y <= button_y + 50.0
                    {
                        game.start_game();
                    }
                }
            }
        }

        game.draw();
        next_frame().await;
    }
}

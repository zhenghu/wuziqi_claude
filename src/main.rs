// 五子棋 (Gomoku) — Rust + macroquad
// 模式: 人机对战 (默认, 玩家执黑) / 双人对战
// 操作: 鼠标点击落子 | R 重开 | U 悔棋 | M 切换模式

mod ai;

use ai::ai_move;
#[cfg(test)]
use ai::{
    double_threat_moves, immediate_winning_moves, line_stat, near_stone, pattern_score, point_score,
};
use macroquad::prelude::*;

const BOARD: usize = 15;
const CENTER: usize = BOARD / 2;
const DIRECTIONS: [(i32, i32); 4] = [(1, 0), (0, 1), (1, 1), (1, -1)];
const CELL: f32 = 40.0;
const MARGIN: f32 = 40.0;
const TOP_BAR: f32 = 70.0;

const BOARD_PX: f32 = CELL * (BOARD as f32 - 1.0);
const WIN_W: f32 = MARGIN * 2.0 + BOARD_PX;
const WIN_H: f32 = TOP_BAR + MARGIN * 2.0 + BOARD_PX;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cell {
    Empty,
    Black,
    White,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    HumanVsAi,
    HumanVsHuman,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Status {
    Playing,
    Won(Cell),
    Draw,
}

struct Game {
    board: [[Cell; BOARD]; BOARD],
    turn: Cell,
    status: Status,
    mode: Mode,
    history: Vec<(usize, usize)>,
    win_line: Vec<(usize, usize)>,
    ai_thinking: bool,
}

impl Game {
    fn new(mode: Mode) -> Self {
        Self {
            board: [[Cell::Empty; BOARD]; BOARD],
            turn: Cell::Black,
            status: Status::Playing,
            mode,
            history: Vec::new(),
            win_line: Vec::new(),
            ai_thinking: false,
        }
    }

    fn place(&mut self, x: usize, y: usize) -> bool {
        if self.status != Status::Playing
            || x >= BOARD
            || y >= BOARD
            || self.board[y][x] != Cell::Empty
        {
            return false;
        }
        self.board[y][x] = self.turn;
        self.history.push((x, y));
        if let Some(line) = winning_line(&self.board, x, y) {
            self.status = Status::Won(self.turn);
            self.win_line = line;
        } else if self.history.len() == BOARD * BOARD {
            self.status = Status::Draw;
        } else {
            self.turn = opponent(self.turn);
        }
        true
    }

    fn undo(&mut self) {
        // 人机模式下，AI 已回应时回退两步；玩家刚落子而 AI 尚未回应时只回退一步。
        // 两种情况最终都回到玩家（黑棋）可落子的局面。
        let steps = match self.mode {
            Mode::HumanVsAi if self.history.len().is_multiple_of(2) => 2,
            _ => 1,
        };
        for _ in 0..steps {
            if let Some((x, y)) = self.history.pop() {
                self.board[y][x] = Cell::Empty;
            }
        }
        // 重算轮次与状态
        self.status = Status::Playing;
        self.win_line.clear();
        self.ai_thinking = false;
        self.turn = if self.history.len().is_multiple_of(2) {
            Cell::Black
        } else {
            Cell::White
        };
    }
}

fn opponent(c: Cell) -> Cell {
    match c {
        Cell::Black => Cell::White,
        Cell::White => Cell::Black,
        Cell::Empty => Cell::Empty,
    }
}

fn in_board(x: i32, y: i32) -> bool {
    x >= 0 && y >= 0 && (x as usize) < BOARD && (y as usize) < BOARD
}

/// 若 (x,y) 落子构成五连，返回首个获胜方向上的所有连续点
fn winning_line(board: &[[Cell; BOARD]; BOARD], x: usize, y: usize) -> Option<Vec<(usize, usize)>> {
    if x >= BOARD || y >= BOARD {
        return None;
    }
    let p = board[y][x];
    if p == Cell::Empty {
        return None;
    }
    for (dx, dy) in DIRECTIONS {
        let mut line = vec![(x, y)];
        for dir in [1i32, -1i32] {
            let (mut cx, mut cy) = (x as i32 + dx * dir, y as i32 + dy * dir);
            while in_board(cx, cy) && board[cy as usize][cx as usize] == p {
                line.push((cx as usize, cy as usize));
                cx += dx * dir;
                cy += dy * dir;
            }
        }
        if line.len() >= 5 {
            return Some(line);
        }
    }
    None
}

// ---------------------------------------------------------------- 绘制

fn board_origin() -> (f32, f32) {
    (MARGIN, TOP_BAR + MARGIN)
}

fn cell_center(x: usize, y: usize) -> (f32, f32) {
    let (ox, oy) = board_origin();
    (ox + x as f32 * CELL, oy + y as f32 * CELL)
}

fn pixel_to_cell(mx: f32, my: f32) -> Option<(usize, usize)> {
    let (ox, oy) = board_origin();
    let fx = (mx - ox) / CELL;
    let fy = (my - oy) / CELL;
    let (rx, ry) = (fx.round(), fy.round());
    if rx < 0.0 || ry < 0.0 || rx >= BOARD as f32 || ry >= BOARD as f32 {
        return None;
    }
    // 点击需足够靠近交叉点
    if (fx - rx).abs() > 0.4 || (fy - ry).abs() > 0.4 {
        return None;
    }
    Some((rx as usize, ry as usize))
}

fn draw_stone(x: usize, y: usize, c: Cell, highlight: bool) {
    let (cx, cy) = cell_center(x, y);
    let r = CELL * 0.42;
    match c {
        Cell::Black => {
            draw_circle(cx, cy, r, Color::from_rgba(20, 20, 20, 255));
            draw_circle(
                cx - r * 0.3,
                cy - r * 0.3,
                r * 0.25,
                Color::from_rgba(90, 90, 90, 200),
            );
        }
        Cell::White => {
            draw_circle(cx, cy, r, Color::from_rgba(240, 240, 240, 255));
            draw_circle_lines(cx, cy, r, 1.0, Color::from_rgba(120, 120, 120, 255));
            draw_circle(cx - r * 0.3, cy - r * 0.3, r * 0.22, WHITE);
        }
        Cell::Empty => {}
    }
    if highlight {
        draw_circle_lines(cx, cy, r * 0.55, 2.0, Color::from_rgba(220, 60, 60, 255));
    }
}

struct Button {
    rect: Rect,
    label: &'static str,
}

impl Button {
    fn new(x: f32, y: f32, w: f32, h: f32, label: &'static str) -> Self {
        Self {
            rect: Rect::new(x, y, w, h),
            label,
        }
    }

    fn draw(&self) -> bool {
        let (mx, my) = mouse_position();
        let hover = self.rect.contains(vec2(mx, my));
        let bg = if hover {
            Color::from_rgba(90, 130, 180, 255)
        } else {
            Color::from_rgba(70, 105, 150, 255)
        };
        draw_rectangle(self.rect.x, self.rect.y, self.rect.w, self.rect.h, bg);
        let ts = measure_text(self.label, None, 20, 1.0);
        draw_text(
            self.label,
            self.rect.x + (self.rect.w - ts.width) / 2.0,
            self.rect.y + (self.rect.h + ts.height) / 2.0 - 2.0,
            20.0,
            WHITE,
        );
        hover && is_mouse_button_pressed(MouseButton::Left)
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Wuziqi - Gomoku".to_owned(),
        window_width: WIN_W as i32,
        window_height: WIN_H as i32,
        window_resizable: false,
        high_dpi: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut game = Game::new(Mode::HumanVsAi);
    let star_points = [(3, 3), (3, 11), (11, 3), (11, 11), (CENTER, CENTER)];

    loop {
        clear_background(Color::from_rgba(40, 44, 52, 255));

        // ---- 顶栏
        draw_rectangle(0.0, 0.0, WIN_W, TOP_BAR, Color::from_rgba(30, 33, 40, 255));
        let btn_mode = Button::new(
            12.0,
            12.0,
            150.0,
            30.0,
            match game.mode {
                Mode::HumanVsAi => "Mode: You vs AI",
                Mode::HumanVsHuman => "Mode: 2 Players",
            },
        );
        let btn_undo = Button::new(174.0, 12.0, 90.0, 30.0, "Undo (U)");
        let btn_restart = Button::new(276.0, 12.0, 110.0, 30.0, "Restart (R)");

        let status_text = match game.status {
            Status::Playing => {
                if game.ai_thinking {
                    "AI is thinking...".to_string()
                } else {
                    match (game.mode, game.turn) {
                        (Mode::HumanVsAi, Cell::Black) => "Your turn (Black)".to_string(),
                        (Mode::HumanVsAi, _) => "AI's turn (White)".to_string(),
                        (_, Cell::Black) => "Black's turn".to_string(),
                        (_, _) => "White's turn".to_string(),
                    }
                }
            }
            Status::Won(Cell::Black) => match game.mode {
                Mode::HumanVsAi => "You win! Press R".to_string(),
                _ => "Black wins! Press R".to_string(),
            },
            Status::Won(_) => match game.mode {
                Mode::HumanVsAi => "AI wins! Press R".to_string(),
                _ => "White wins! Press R".to_string(),
            },
            Status::Draw => "Draw! Press R".to_string(),
        };
        draw_text(
            &status_text,
            14.0,
            TOP_BAR - 12.0,
            24.0,
            Color::from_rgba(255, 210, 120, 255),
        );

        // ---- 棋盘
        let (ox, oy) = board_origin();
        draw_rectangle(
            ox - CELL * 0.5,
            oy - CELL * 0.5,
            BOARD_PX + CELL,
            BOARD_PX + CELL,
            Color::from_rgba(210, 168, 110, 255),
        );
        for i in 0..BOARD {
            let t = i as f32 * CELL;
            draw_line(
                ox,
                oy + t,
                ox + BOARD_PX,
                oy + t,
                1.2,
                Color::from_rgba(60, 40, 20, 255),
            );
            draw_line(
                ox + t,
                oy,
                ox + t,
                oy + BOARD_PX,
                1.2,
                Color::from_rgba(60, 40, 20, 255),
            );
        }
        for &(sx, sy) in &star_points {
            let (cx, cy) = cell_center(sx, sy);
            draw_circle(cx, cy, 4.0, Color::from_rgba(60, 40, 20, 255));
        }

        // ---- 棋子
        let last = game.history.last().copied();
        for y in 0..BOARD {
            for x in 0..BOARD {
                if game.board[y][x] != Cell::Empty {
                    draw_stone(x, y, game.board[y][x], last == Some((x, y)));
                }
            }
        }
        // 胜利连线高亮
        for &(x, y) in &game.win_line {
            let (cx, cy) = cell_center(x, y);
            draw_circle_lines(
                cx,
                cy,
                CELL * 0.46,
                3.0,
                Color::from_rgba(60, 220, 100, 255),
            );
        }

        // ---- 悬停预览
        let human_turn = game.status == Status::Playing
            && !(game.mode == Mode::HumanVsAi && game.turn == Cell::White);
        if human_turn {
            let (mx, my) = mouse_position();
            if let Some((hx, hy)) = pixel_to_cell(mx, my) {
                if game.board[hy][hx] == Cell::Empty {
                    let (cx, cy) = cell_center(hx, hy);
                    let col = match game.turn {
                        Cell::Black => Color::from_rgba(20, 20, 20, 110),
                        _ => Color::from_rgba(250, 250, 250, 140),
                    };
                    draw_circle(cx, cy, CELL * 0.42, col);
                }
            }
        }

        // ---- 输入处理
        let mut restart = is_key_pressed(KeyCode::R);
        let mut undo = is_key_pressed(KeyCode::U);
        let mut toggle_mode = is_key_pressed(KeyCode::M);
        if btn_restart.draw() {
            restart = true;
        }
        if btn_undo.draw() {
            undo = true;
        }
        if btn_mode.draw() {
            toggle_mode = true;
        }

        if toggle_mode {
            let new_mode = match game.mode {
                Mode::HumanVsAi => Mode::HumanVsHuman,
                Mode::HumanVsHuman => Mode::HumanVsAi,
            };
            game = Game::new(new_mode);
        } else if restart {
            game = Game::new(game.mode);
        } else if undo {
            game.undo();
        } else if game.status == Status::Playing {
            if game.mode == Mode::HumanVsAi && game.turn == Cell::White {
                // 先渲染一帧 "AI thinking" 再计算
                if game.ai_thinking {
                    let (ax, ay) = ai_move(&game.board, Cell::White, game.history.len());
                    game.place(ax, ay);
                    game.ai_thinking = false;
                } else {
                    game.ai_thinking = true;
                }
            } else if human_turn && is_mouse_button_pressed(MouseButton::Left) {
                let (mx, my) = mouse_position();
                // 避免点击顶栏按钮时误落子
                if my > TOP_BAR {
                    if let Some((cx, cy)) = pixel_to_cell(mx, my) {
                        game.place(cx, cy);
                    }
                }
            }
        }

        next_frame().await
    }
}

#[cfg(test)]
#[path = "../tests/unit_tests/mod.rs"]
mod tests;

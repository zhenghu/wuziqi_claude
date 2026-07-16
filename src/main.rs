// 五子棋 (Gomoku) — Rust + macroquad
// 模式: 人机对战 (默认, 玩家执黑) / 双人对战
// 操作: 鼠标点击落子 | R 重开 | U 悔棋 | M 切换模式

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

// ---------------------------------------------------------------- AI

const ROOT_CANDIDATE_LIMIT: usize = 12;
const REPLY_CANDIDATE_LIMIT: usize = 10;
const MATE_SCORE: i64 = 1_000_000_000_000;

/// 在 (x,y) 为 p 落子后, 单方向的 (连子数, 开放端数)
fn line_stat(
    board: &[[Cell; BOARD]; BOARD],
    x: usize,
    y: usize,
    dx: i32,
    dy: i32,
    p: Cell,
) -> (u32, u32) {
    let mut count = 1u32;
    let mut open = 0u32;
    for dir in [1i32, -1i32] {
        let (mut cx, mut cy) = (x as i32 + dx * dir, y as i32 + dy * dir);
        while in_board(cx, cy) && board[cy as usize][cx as usize] == p {
            count += 1;
            cx += dx * dir;
            cy += dy * dir;
        }
        if in_board(cx, cy) && board[cy as usize][cx as usize] == Cell::Empty {
            open += 1;
        }
    }
    (count, open)
}

fn pattern_score(count: u32, open: u32) -> i64 {
    match (count, open) {
        (c, _) if c >= 5 => 10_000_000,
        (4, 2) => 1_000_000,
        (4, 1) => 120_000,
        (3, 2) => 60_000,
        (3, 1) => 2_000,
        (2, 2) => 800,
        (2, 1) => 150,
        (1, 2) => 40,
        (1, 1) => 10,
        _ => 0,
    }
}

fn point_score(board: &[[Cell; BOARD]; BOARD], x: usize, y: usize, p: Cell) -> i64 {
    let mut total = 0i64;
    for (dx, dy) in DIRECTIONS {
        let (count, open) = line_stat(board, x, y, dx, dy, p);
        total += pattern_score(count, open);
    }
    total
}

/// 只考虑已有棋子周围 2 格内的空位
fn near_stone(board: &[[Cell; BOARD]; BOARD], x: usize, y: usize) -> bool {
    for dy in -2i32..=2 {
        for dx in -2i32..=2 {
            let (cx, cy) = (x as i32 + dx, y as i32 + dy);
            if in_board(cx, cy) && board[cy as usize][cx as usize] != Cell::Empty {
                return true;
            }
        }
    }
    false
}

fn candidate_moves(board: &[[Cell; BOARD]; BOARD]) -> Vec<(usize, usize)> {
    let mut moves = Vec::new();
    for y in 0..BOARD {
        for x in 0..BOARD {
            if board[y][x] == Cell::Empty && near_stone(board, x, y) {
                moves.push((x, y));
            }
        }
    }
    if moves.is_empty() && board[CENTER][CENTER] == Cell::Empty {
        moves.push((CENTER, CENTER));
    }
    moves
}

fn is_winning_move(board: &[[Cell; BOARD]; BOARD], x: usize, y: usize, p: Cell) -> bool {
    board[y][x] == Cell::Empty
        && DIRECTIONS
            .iter()
            .any(|&(dx, dy)| line_stat(board, x, y, dx, dy, p).0 >= 5)
}

fn immediate_winning_moves(board: &[[Cell; BOARD]; BOARD], p: Cell) -> Vec<(usize, usize)> {
    candidate_moves(board)
        .into_iter()
        .filter(|&(x, y)| is_winning_move(board, x, y, p))
        .collect()
}

fn count_immediate_wins(board: &[[Cell; BOARD]; BOARD], p: Cell, stop_after: usize) -> usize {
    let mut count = 0;
    for (x, y) in candidate_moves(board) {
        if is_winning_move(board, x, y, p) {
            count += 1;
            if count == stop_after {
                break;
            }
        }
    }
    count
}

/// 返回落子后能同时产生至少两个立即获胜点的位置（双杀）。
fn double_threat_moves(board: &[[Cell; BOARD]; BOARD], p: Cell) -> Vec<(usize, usize)> {
    let mut threats = Vec::new();
    for (x, y) in candidate_moves(board) {
        if is_winning_move(board, x, y, p) {
            continue;
        }
        let mut next = *board;
        next[y][x] = p;
        if count_immediate_wins(&next, p, 2) >= 2 {
            threats.push((x, y));
        }
    }
    threats
}

fn move_order_score(board: &[[Cell; BOARD]; BOARD], x: usize, y: usize, p: Cell) -> i64 {
    let attack = point_score(board, x, y, p);
    let defend = point_score(board, x, y, opponent(p));
    let center_bias =
        (BOARD - 1) as i64 - ((x as i64 - CENTER as i64).abs() + (y as i64 - CENTER as i64).abs());
    attack * 10 + defend * 9 + center_bias
}

/// 必选战术点排在最前，随后追加评分最高的普通候选。
fn ranked_moves(
    board: &[[Cell; BOARD]; BOARD],
    p: Cell,
    limit: usize,
    required: &[(usize, usize)],
) -> Vec<(usize, usize)> {
    let mut scored: Vec<_> = candidate_moves(board)
        .into_iter()
        .map(|(x, y)| ((x, y), move_order_score(board, x, y, p)))
        .collect();
    scored.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then_with(|| a.0 .1.cmp(&b.0 .1))
            .then_with(|| a.0 .0.cmp(&b.0 .0))
    });

    let mut selected = Vec::new();
    for &((x, y), _) in &scored {
        if required.contains(&(x, y)) {
            selected.push((x, y));
        }
    }
    let mut regular = 0;
    for &((x, y), _) in &scored {
        if selected.contains(&(x, y)) {
            continue;
        }
        if regular == limit {
            break;
        }
        selected.push((x, y));
        regular += 1;
    }
    selected
}

fn threat_value(board: &[[Cell; BOARD]; BOARD], p: Cell) -> i64 {
    let (mut first, mut second) = (0, 0);
    for (x, y) in candidate_moves(board) {
        let score = point_score(board, x, y, p);
        if score > first {
            second = first;
            first = score;
        } else if score > second {
            second = score;
        }
    }
    first * 4 + second
}

fn evaluate_position(board: &[[Cell; BOARD]; BOARD], ai: Cell) -> i64 {
    threat_value(board, ai) * 10 - threat_value(board, opponent(ai)) * 9
}

/// 玩家回应后轮到 AI：识别下一手必胜/必败，否则做静态威胁评估。
fn leaf_score(board: &[[Cell; BOARD]; BOARD], ai: Cell) -> i64 {
    let ai_wins = immediate_winning_moves(board, ai);
    if !ai_wins.is_empty() {
        return MATE_SCORE;
    }

    let human = opponent(ai);
    let human_wins = immediate_winning_moves(board, human);
    if human_wins.len() >= 2 {
        return -MATE_SCORE;
    }
    if let Some(&(x, y)) = human_wins.first() {
        // 唯一直接威胁可以被 AI 下一手强制挡住，延伸这一手再评估。
        let mut forced = *board;
        forced[y][x] = ai;
        return evaluate_position(&forced, ai);
    }
    evaluate_position(board, ai)
}

/// 在给定 AI 根着后，计算玩家最佳回应（极小层）。
fn reply_score(
    board: &[[Cell; BOARD]; BOARD],
    ai: Cell,
    alpha: i64,
    human_forks: &[(usize, usize)],
) -> i64 {
    let human = opponent(ai);
    if !immediate_winning_moves(board, human).is_empty() {
        return -MATE_SCORE;
    }

    let ai_wins = immediate_winning_moves(board, ai);
    if ai_wins.len() >= 2 {
        return MATE_SCORE;
    }

    let replies = if ai_wins.len() == 1 {
        // 玩家必须占据 AI 唯一的下一手获胜点。
        ai_wins
    } else {
        // AI 新增的棋子只会消除既有玩家双杀，不会创造新的玩家双杀。
        ranked_moves(board, human, REPLY_CANDIDATE_LIMIT, human_forks)
    };
    if replies.is_empty() {
        return evaluate_position(board, ai);
    }

    let mut worst = MATE_SCORE * 2;
    for (x, y) in replies {
        let mut next = *board;
        next[y][x] = human;
        let score = if winning_line(&next, x, y).is_some() {
            -MATE_SCORE
        } else {
            leaf_score(&next, ai)
        };
        worst = worst.min(score);
        if worst <= alpha {
            break;
        }
    }
    worst
}

fn ai_move(board: &[[Cell; BOARD]; BOARD], ai: Cell, move_count: usize) -> (usize, usize) {
    // 开局下中心。
    if move_count == 0 {
        return (CENTER, CENTER);
    }
    let human = opponent(ai);

    // 立即获胜永远优先于防守。
    let ai_wins = immediate_winning_moves(board, ai);
    if !ai_wins.is_empty() {
        return ranked_moves(board, ai, 0, &ai_wins)[0];
    }

    // 对手有直接胜点时必须占据；多个胜点已无法全防，选择其中评分最高者。
    let human_wins = immediate_winning_moves(board, human);
    if !human_wins.is_empty() {
        return ranked_moves(board, ai, 0, &human_wins)[0];
    }

    // 一步制造两个立即胜点即为强制胜。
    let ai_forks = double_threat_moves(board, ai);
    if !ai_forks.is_empty() {
        return ranked_moves(board, ai, 0, &ai_forks)[0];
    }

    // 把对手潜在双杀点强制并入根候选，避免被限宽剪掉。
    let human_forks = double_threat_moves(board, human);
    if human_forks.len() == 1 {
        return human_forks[0];
    }
    let roots = ranked_moves(board, ai, ROOT_CANDIDATE_LIMIT, &human_forks);
    let Some(&mut_best) = roots.first() else {
        return (CENTER, CENTER);
    };
    let mut best = mut_best;
    let mut best_score = -MATE_SCORE * 2;
    for (x, y) in roots {
        let mut next = *board;
        next[y][x] = ai;
        let score = reply_score(&next, ai, best_score, &human_forks);
        if score > best_score {
            best_score = score;
            best = (x, y);
        }
    }
    best
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

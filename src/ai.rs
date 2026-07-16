//! 五子棋的落点评分、战术识别与限宽搜索。

use crate::{in_board, opponent, winning_line, Cell, BOARD, CENTER, DIRECTIONS};

const ROOT_CANDIDATE_LIMIT: usize = 12;
const REPLY_CANDIDATE_LIMIT: usize = 10;
const MATE_SCORE: i64 = 1_000_000_000_000;

/// 在 (x,y) 为 p 落子后, 单方向的 (连子数, 开放端数)
pub(crate) fn line_stat(
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

pub(crate) fn pattern_score(count: u32, open: u32) -> i64 {
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

pub(crate) fn point_score(board: &[[Cell; BOARD]; BOARD], x: usize, y: usize, p: Cell) -> i64 {
    let mut total = 0i64;
    for (dx, dy) in DIRECTIONS {
        let (count, open) = line_stat(board, x, y, dx, dy, p);
        total += pattern_score(count, open);
    }
    total
}

/// 只考虑已有棋子周围 2 格内的空位
pub(crate) fn near_stone(board: &[[Cell; BOARD]; BOARD], x: usize, y: usize) -> bool {
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

pub(crate) fn immediate_winning_moves(
    board: &[[Cell; BOARD]; BOARD],
    p: Cell,
) -> Vec<(usize, usize)> {
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
pub(crate) fn double_threat_moves(board: &[[Cell; BOARD]; BOARD], p: Cell) -> Vec<(usize, usize)> {
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

pub(crate) fn ai_move(
    board: &[[Cell; BOARD]; BOARD],
    ai: Cell,
    move_count: usize,
) -> (usize, usize) {
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

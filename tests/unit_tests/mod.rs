use super::*;

type Board = [[Cell; BOARD]; BOARD];

fn empty_board() -> Board {
    [[Cell::Empty; BOARD]; BOARD]
}

fn put(board: &mut Board, stones: &[(usize, usize)], cell: Cell) {
    for &(x, y) in stones {
        board[y][x] = cell;
    }
}

fn play_black_horizontal_win(mode: Mode) -> Game {
    let mut game = Game::new(mode);
    for &(x, y) in &[
        (3, 7),
        (0, 0),
        (4, 7),
        (0, 1),
        (5, 7),
        (0, 2),
        (6, 7),
        (0, 3),
        (7, 7),
    ] {
        assert!(game.place(x, y));
    }
    game
}

#[test]
fn opponent_swaps_colors_and_keeps_empty() {
    assert_eq!(opponent(Cell::Black), Cell::White);
    assert_eq!(opponent(Cell::White), Cell::Black);
    assert_eq!(opponent(Cell::Empty), Cell::Empty);
}

#[test]
fn winning_line_detects_all_four_axes() {
    for (dx, dy) in [(1, 0), (0, 1), (1, 1), (1, -1)] {
        let mut board = empty_board();
        let expected: Vec<_> = (-2..=2)
            .map(|offset| ((7i32 + dx * offset) as usize, (7i32 + dy * offset) as usize))
            .collect();
        put(&mut board, &expected, Cell::Black);

        let line = winning_line(&board, 7, 7).expect("five stones should win");
        assert_eq!(line.len(), 5);
        assert!(expected.iter().all(|point| line.contains(point)));
    }
}

#[test]
fn winning_line_rejects_a_gap_and_accepts_an_overline() {
    let mut broken = empty_board();
    put(&mut broken, &[(3, 7), (4, 7), (6, 7), (7, 7)], Cell::Black);
    assert!(winning_line(&broken, 3, 7).is_none());

    let mut overline = empty_board();
    let six: Vec<_> = (2..=7).map(|x| (x, 7)).collect();
    put(&mut overline, &six, Cell::White);
    let line = winning_line(&overline, 5, 7).expect("six stones should also win");
    assert_eq!(line.len(), 6);
}

#[test]
fn line_stat_counts_contiguous_stones_and_open_ends() {
    let mut board = empty_board();
    put(&mut board, &[(5, 7), (6, 7), (8, 7)], Cell::Black);
    assert_eq!(line_stat(&board, 7, 7, 1, 0, Cell::Black), (4, 2));

    board[7][9] = Cell::White;
    assert_eq!(line_stat(&board, 7, 7, 1, 0, Cell::Black), (4, 1));

    let mut edge = empty_board();
    edge[0][1] = Cell::Black;
    assert_eq!(line_stat(&edge, 0, 0, 1, 0, Cell::Black), (2, 1));
}

#[test]
fn pattern_score_table_is_stable() {
    let cases = [
        ((5, 0), 10_000_000),
        ((6, 2), 10_000_000),
        ((4, 2), 1_000_000),
        ((4, 1), 120_000),
        ((3, 2), 60_000),
        ((3, 1), 2_000),
        ((2, 2), 800),
        ((2, 1), 150),
        ((1, 2), 40),
        ((1, 1), 10),
        ((4, 0), 0),
        ((0, 2), 0),
    ];

    for ((count, open), expected) in cases {
        assert_eq!(pattern_score(count, open), expected);
    }
}

#[test]
fn point_score_sums_patterns_across_axes() {
    let mut board = empty_board();
    put(&mut board, &[(6, 7), (8, 7), (7, 6), (7, 8)], Cell::Black);

    assert_eq!(point_score(&board, 7, 7, Cell::Black), 120_080);
}

#[test]
fn near_stone_uses_a_two_cell_square_radius() {
    let mut board = empty_board();
    assert!(!near_stone(&board, 7, 7));

    board[7][7] = Cell::Black;
    assert!(near_stone(&board, 9, 9));
    assert!(!near_stone(&board, 10, 7));
}

#[test]
fn ai_opens_at_the_center() {
    assert_eq!(ai_move(&empty_board(), Cell::White, 0), (CENTER, CENTER));
}

#[test]
fn ai_takes_a_unique_immediate_win() {
    let mut board = empty_board();
    put(&mut board, &[(0, 7), (1, 7), (2, 7), (3, 7)], Cell::White);

    assert_eq!(ai_move(&board, Cell::White, 4), (4, 7));
}

#[test]
fn ai_blocks_a_unique_immediate_loss() {
    let mut board = empty_board();
    put(&mut board, &[(0, 7), (1, 7), (2, 7), (3, 7)], Cell::Black);

    assert_eq!(ai_move(&board, Cell::White, 4), (4, 7));
}

#[test]
fn ai_prefers_its_own_win_over_blocking() {
    let mut board = empty_board();
    put(&mut board, &[(0, 7), (1, 7), (2, 7), (3, 7)], Cell::White);
    put(&mut board, &[(0, 8), (1, 8), (2, 8), (3, 8)], Cell::Black);

    assert_eq!(ai_move(&board, Cell::White, 8), (4, 7));
}

#[test]
fn ai_is_deterministic_and_returns_a_legal_move() {
    let mut board = empty_board();
    board[7][7] = Cell::Black;
    let before = board;

    let first = ai_move(&board, Cell::White, 1);
    let second = ai_move(&board, Cell::White, 1);
    assert_eq!(first, second);
    assert_eq!(board[first.1][first.0], Cell::Empty);
    assert_eq!(board, before);
}

#[test]
fn llm_candidates_are_legal_and_preserve_forced_defense() {
    let mut board = empty_board();
    put(&mut board, &[(0, 7), (1, 7), (2, 7), (3, 7)], Cell::Black);

    let candidates = llm_candidate_moves(&board, Cell::White, 4);
    assert_eq!(candidates, vec![(4, 7)]);
    assert!(candidates
        .iter()
        .all(|&(x, y)| x < BOARD && y < BOARD && board[y][x] == Cell::Empty));
}

#[test]
fn immediate_winning_moves_finds_both_open_four_ends() {
    let mut board = empty_board();
    put(&mut board, &[(5, 7), (6, 7), (7, 7), (8, 7)], Cell::White);

    assert_eq!(
        immediate_winning_moves(&board, Cell::White),
        vec![(4, 7), (9, 7)]
    );
}

#[test]
fn ai_creates_a_forced_win_with_a_double_threat() {
    let mut board = empty_board();
    put(
        &mut board,
        &[(4, 7), (5, 7), (8, 7), (7, 4), (7, 5), (7, 8)],
        Cell::White,
    );
    put(
        &mut board,
        &[
            (10, 10),
            (11, 10),
            (0, 0),
            (14, 14),
            (0, 14),
            (14, 0),
            (1, 12),
        ],
        Cell::Black,
    );

    assert_eq!(double_threat_moves(&board, Cell::White), vec![(7, 7)]);
    let chosen = ai_move(&board, Cell::White, 13);
    assert_eq!(chosen, (7, 7));

    board[chosen.1][chosen.0] = Cell::White;
    assert_eq!(
        immediate_winning_moves(&board, Cell::White),
        vec![(7, 6), (6, 7)]
    );
}

#[test]
fn ai_prevents_the_opponents_next_move_double_threat() {
    let mut board = empty_board();
    put(
        &mut board,
        &[(4, 7), (5, 7), (8, 7), (7, 4), (7, 5), (7, 8)],
        Cell::Black,
    );
    put(
        &mut board,
        &[(10, 10), (11, 10), (0, 0), (14, 14), (0, 14)],
        Cell::White,
    );

    assert_eq!(double_threat_moves(&board, Cell::Black), vec![(7, 7)]);
    let chosen = ai_move(&board, Cell::White, 11);
    assert!([(7, 7), (6, 7), (7, 6)].contains(&chosen));

    board[chosen.1][chosen.0] = Cell::White;
    assert!(double_threat_moves(&board, Cell::Black).is_empty());
}

#[test]
fn place_updates_state_and_rejects_an_occupied_cell() {
    let mut game = Game::new(Mode::HumanVsHuman);

    assert!(game.place(7, 7));
    assert_eq!(game.board[7][7], Cell::Black);
    assert_eq!(game.history, vec![(7, 7)]);
    assert_eq!(game.turn, Cell::White);
    assert_eq!(game.status, Status::Playing);

    assert!(!game.place(7, 7));
    assert_eq!(game.history, vec![(7, 7)]);
    assert_eq!(game.turn, Cell::White);
}

#[test]
fn place_rejects_out_of_bounds_without_mutating_state() {
    let mut game = Game::new(Mode::HumanVsHuman);
    let board_before = game.board;

    assert!(!game.place(BOARD, 0));
    assert!(!game.place(0, BOARD));
    assert_eq!(game.board, board_before);
    assert!(game.history.is_empty());
    assert_eq!(game.turn, Cell::Black);
    assert_eq!(game.status, Status::Playing);

    assert!(winning_line(&game.board, BOARD, 0).is_none());
    assert!(winning_line(&game.board, 0, BOARD).is_none());
}

#[test]
fn place_transitions_to_won_and_rejects_later_moves() {
    let mut game = play_black_horizontal_win(Mode::HumanVsHuman);

    assert_eq!(game.status, Status::Won(Cell::Black));
    assert_eq!(game.turn, Cell::Black);
    assert_eq!(game.win_line.len(), 5);
    assert!(!game.place(8, 8));
    assert_eq!(game.history.len(), 9);
}

#[test]
fn undo_restores_invariants_in_both_modes() {
    let mut pvp = play_black_horizontal_win(Mode::HumanVsHuman);
    pvp.ai_thinking = true;
    pvp.undo();
    assert_eq!(pvp.status, Status::Playing);
    assert_eq!(pvp.turn, Cell::Black);
    assert_eq!(pvp.history.len(), 8);
    assert_eq!(pvp.board[7][7], Cell::Empty);
    assert!(pvp.win_line.is_empty());
    assert!(!pvp.ai_thinking);

    let mut versus_ai = Game::new(Mode::HumanVsAi);
    assert!(versus_ai.place(7, 7));
    assert!(versus_ai.place(7, 8));
    versus_ai.ai_thinking = true;
    versus_ai.undo();
    assert!(versus_ai.history.is_empty());
    assert_eq!(versus_ai.board[7][7], Cell::Empty);
    assert_eq!(versus_ai.board[8][7], Cell::Empty);
    assert_eq!(versus_ai.turn, Cell::Black);
    assert_eq!(versus_ai.status, Status::Playing);
    assert!(!versus_ai.ai_thinking);
}

#[test]
fn undo_before_ai_reply_only_removes_the_latest_human_move() {
    let mut game = Game::new(Mode::HumanVsAi);
    assert!(game.place(7, 7));
    assert!(game.place(7, 8));
    assert!(game.place(8, 7));
    game.ai_thinking = true;

    game.undo();

    assert_eq!(game.history, vec![(7, 7), (7, 8)]);
    assert_eq!(game.board[7][7], Cell::Black);
    assert_eq!(game.board[8][7], Cell::White);
    assert_eq!(game.board[7][8], Cell::Empty);
    assert_eq!(game.turn, Cell::Black);
    assert_eq!(game.status, Status::Playing);
    assert!(game.win_line.is_empty());
    assert!(!game.ai_thinking);
}

#[test]
fn undo_after_a_human_win_only_removes_the_winning_move() {
    let mut game = play_black_horizontal_win(Mode::HumanVsAi);

    game.undo();

    assert_eq!(game.history.len(), 8);
    assert_eq!(game.board[7][7], Cell::Empty);
    assert_eq!(game.turn, Cell::Black);
    assert_eq!(game.status, Status::Playing);
    assert!(game.win_line.is_empty());
}

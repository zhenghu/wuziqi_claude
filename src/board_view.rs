//! 棋盘坐标换算和绘制。

use crate::game::{Cell, Game, BOARD, CENTER};
use macroquad::prelude::*;

pub(crate) const CELL: f32 = 40.0;
pub(crate) const MARGIN: f32 = 40.0;
pub(crate) const TOP_BAR: f32 = 70.0;
pub(crate) const BOARD_PX: f32 = CELL * (BOARD as f32 - 1.0);
pub(crate) const WIN_W: f32 = MARGIN * 2.0 + BOARD_PX;
pub(crate) const WIN_H: f32 = TOP_BAR + MARGIN * 2.0 + BOARD_PX;

pub(crate) fn window_conf() -> Conf {
    Conf {
        window_title: "Wuziqi - Gomoku".to_owned(),
        window_width: WIN_W as i32,
        window_height: WIN_H as i32,
        window_resizable: false,
        high_dpi: true,
        ..Default::default()
    }
}

fn board_origin() -> (f32, f32) {
    (MARGIN, TOP_BAR + MARGIN)
}

pub(crate) fn cell_center(x: usize, y: usize) -> (f32, f32) {
    let (ox, oy) = board_origin();
    (ox + x as f32 * CELL, oy + y as f32 * CELL)
}

pub(crate) fn pixel_to_cell(mx: f32, my: f32) -> Option<(usize, usize)> {
    let (ox, oy) = board_origin();
    let fx = (mx - ox) / CELL;
    let fy = (my - oy) / CELL;
    let (rx, ry) = (fx.round(), fy.round());
    if rx < 0.0 || ry < 0.0 || rx >= BOARD as f32 || ry >= BOARD as f32 {
        return None;
    }
    if (fx - rx).abs() > 0.4 || (fy - ry).abs() > 0.4 {
        return None;
    }
    Some((rx as usize, ry as usize))
}

pub(crate) fn draw(game: &Game, human_turn: bool) {
    let (ox, oy) = board_origin();
    draw_rectangle(
        ox - CELL * 0.5,
        oy - CELL * 0.5,
        BOARD_PX + CELL,
        BOARD_PX + CELL,
        Color::from_rgba(210, 168, 110, 255),
    );
    for i in 0..BOARD {
        let offset = i as f32 * CELL;
        draw_line(
            ox,
            oy + offset,
            ox + BOARD_PX,
            oy + offset,
            1.2,
            Color::from_rgba(60, 40, 20, 255),
        );
        draw_line(
            ox + offset,
            oy,
            ox + offset,
            oy + BOARD_PX,
            1.2,
            Color::from_rgba(60, 40, 20, 255),
        );
    }
    for &(x, y) in &[(3, 3), (3, 11), (11, 3), (11, 11), (CENTER, CENTER)] {
        let (cx, cy) = cell_center(x, y);
        draw_circle(cx, cy, 4.0, Color::from_rgba(60, 40, 20, 255));
    }

    let last = game.history.last().copied();
    for y in 0..BOARD {
        for x in 0..BOARD {
            if game.board[y][x] != Cell::Empty {
                draw_stone(x, y, game.board[y][x], last == Some((x, y)));
            }
        }
    }
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

    if human_turn {
        let (mx, my) = mouse_position();
        if let Some((x, y)) = pixel_to_cell(mx, my) {
            if game.board[y][x] == Cell::Empty {
                let (cx, cy) = cell_center(x, y);
                let color = match game.turn {
                    Cell::Black => Color::from_rgba(20, 20, 20, 110),
                    _ => Color::from_rgba(250, 250, 250, 140),
                };
                draw_circle(cx, cy, CELL * 0.42, color);
            }
        }
    }
}

fn draw_stone(x: usize, y: usize, cell: Cell, highlight: bool) {
    let (cx, cy) = cell_center(x, y);
    let radius = CELL * 0.42;
    match cell {
        Cell::Black => {
            draw_circle(cx, cy, radius, Color::from_rgba(20, 20, 20, 255));
            draw_circle(
                cx - radius * 0.3,
                cy - radius * 0.3,
                radius * 0.25,
                Color::from_rgba(90, 90, 90, 200),
            );
        }
        Cell::White => {
            draw_circle(cx, cy, radius, Color::from_rgba(240, 240, 240, 255));
            draw_circle_lines(cx, cy, radius, 1.0, Color::from_rgba(120, 120, 120, 255));
            draw_circle(cx - radius * 0.3, cy - radius * 0.3, radius * 0.22, WHITE);
        }
        Cell::Empty => {}
    }
    if highlight {
        draw_circle_lines(
            cx,
            cy,
            radius * 0.55,
            2.0,
            Color::from_rgba(220, 60, 60, 255),
        );
    }
}

pub(crate) struct Button {
    rect: Rect,
    label: &'static str,
}

impl Button {
    pub(crate) fn new(x: f32, y: f32, width: f32, height: f32, label: &'static str) -> Self {
        Self {
            rect: Rect::new(x, y, width, height),
            label,
        }
    }

    pub(crate) fn draw(&self) -> bool {
        let (mx, my) = mouse_position();
        let hover = self.rect.contains(vec2(mx, my));
        let color = if hover {
            Color::from_rgba(90, 130, 180, 255)
        } else {
            Color::from_rgba(70, 105, 150, 255)
        };
        draw_rectangle(self.rect.x, self.rect.y, self.rect.w, self.rect.h, color);
        let size = measure_text(self.label, None, 20, 1.0);
        draw_text(
            self.label,
            self.rect.x + (self.rect.w - size.width) / 2.0,
            self.rect.y + (self.rect.h + size.height) / 2.0 - 2.0,
            20.0,
            WHITE,
        );
        hover && is_mouse_button_pressed(MouseButton::Left)
    }
}

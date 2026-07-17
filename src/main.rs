mod ai;
mod app;
mod board_view;
mod config_ui;
mod game;
mod llm_ai;

use app::App;
use board_view::window_conf;

#[macroquad::main(window_conf)]
async fn main() {
    App::new().run().await;
}

#[cfg(test)]
#[path = "../tests/unit_tests/mod.rs"]
mod tests;

mod app;
mod theme;
mod tui;
mod ui;

use anyhow::Result;
use app::App;
use tui::run_tui;

fn main() -> Result<()> {
    let mut app = App::new();
    run_tui(&mut app)
}

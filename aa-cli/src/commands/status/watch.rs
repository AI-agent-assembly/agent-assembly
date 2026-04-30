//! Watch mode — auto-refresh status display every 5 seconds.

use std::io::{self, Write};

use crossterm::cursor::MoveTo;
use crossterm::execute;
use crossterm::terminal::{Clear, ClearType};

use tokio::time::{self, Duration};

use super::client::StatusClient;
use super::fetch;
use super::render;
use crate::output::OutputFormat;

/// Clear the terminal screen without flickering.
fn clear_screen() {
    let _ = execute!(io::stdout(), Clear(ClearType::All), MoveTo(0, 0));
    let _ = io::stdout().flush();
}

/// Run the watch loop: fetch, render, clear, repeat every 5 seconds.
///
/// Continues until the process is killed (e.g. Ctrl+C).
pub async fn run_watch_loop(client: &StatusClient, output: OutputFormat) {
    loop {
        clear_screen();
        let snapshot = fetch::fetch_all(client).await;
        render::render_all(&snapshot, output);
        eprintln!("(refreshing every 5s — press Ctrl+C to stop)");
        time::sleep(Duration::from_secs(5)).await;
    }
}

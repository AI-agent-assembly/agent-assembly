//! Watch mode — auto-refresh status display every 5 seconds.

use std::io::{self, Write};

use crossterm::cursor::MoveTo;
use crossterm::execute;
use crossterm::terminal::{Clear, ClearType};

use super::client::StatusClient;
use crate::output::OutputFormat;

/// Clear the terminal screen without flickering.
fn clear_screen() {
    let _ = execute!(io::stdout(), Clear(ClearType::All), MoveTo(0, 0));
    let _ = io::stdout().flush();
}

/// Run the watch loop: fetch, render, clear, repeat every 5 seconds.
pub async fn run_watch_loop(_client: &StatusClient, _output: OutputFormat) {
    eprintln!("watch: not yet implemented");
}

//! Watch mode — auto-refresh status display every 5 seconds.

use super::client::StatusClient;
use crate::output::OutputFormat;

/// Run the watch loop: fetch, render, clear, repeat every 5 seconds.
pub async fn run_watch_loop(_client: &StatusClient, _output: OutputFormat) {
    eprintln!("watch: not yet implemented");
}

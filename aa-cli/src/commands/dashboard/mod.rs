//! `aasm dashboard` — interactive TUI dashboard for real-time governance monitoring.

pub mod feed;
pub mod input;
pub mod state;
pub mod ui;

use std::io::{self, stdout};
use std::process::ExitCode;

use clap::Args;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::config::ResolvedContext;

/// Arguments for the `aasm dashboard` subcommand.
#[derive(Debug, Args)]
pub struct DashboardArgs {}

/// Entry point for `aasm dashboard`.
pub fn dispatch(args: DashboardArgs, ctx: &ResolvedContext) -> ExitCode {
    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    rt.block_on(async { run(args, ctx).await })
}

/// Set up the terminal, run the dashboard, and restore the terminal on exit.
async fn run(_args: DashboardArgs, _ctx: &ResolvedContext) -> ExitCode {
    // Install a panic hook that restores the terminal before printing the panic.
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = restore_terminal();
        original_hook(info);
    }));

    if let Err(e) = setup_terminal() {
        eprintln!("error: failed to initialise terminal: {e}");
        return ExitCode::FAILURE;
    }

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = match Terminal::new(backend) {
        Ok(t) => t,
        Err(e) => {
            let _ = restore_terminal();
            eprintln!("error: failed to create terminal: {e}");
            return ExitCode::FAILURE;
        }
    };

    terminal.clear().ok();

    // TODO: run the main event loop here (Task 7).

    let _ = restore_terminal();
    ExitCode::SUCCESS
}

/// Enter raw mode and alternate screen for the TUI.
fn setup_terminal() -> io::Result<()> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen, EnableMouseCapture)?;
    Ok(())
}

/// Restore the terminal to its original state.
fn restore_terminal() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}

//! `aasm dashboard` — interactive TUI dashboard for real-time governance monitoring.

pub mod feed;
pub mod input;
pub mod state;
pub mod ui;

use std::io::{self, stdout};
use std::process::ExitCode;
use std::time::Duration;

use clap::Args;
use crossterm::event::{self as ct_event, DisableMouseCapture, EnableMouseCapture, Event};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::mpsc;

use crate::config::ResolvedContext;

use self::feed::FeedMessage;
use self::input::InputAction;
use self::state::DashboardState;

/// Arguments for the `aasm dashboard` subcommand.
#[derive(Debug, Args)]
pub struct DashboardArgs {}

/// Entry point for `aasm dashboard`.
pub fn dispatch(args: DashboardArgs, ctx: &ResolvedContext) -> ExitCode {
    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    rt.block_on(async { run(args, ctx).await })
}

/// Set up the terminal, run the dashboard, and restore the terminal on exit.
async fn run(_args: DashboardArgs, ctx: &ResolvedContext) -> ExitCode {
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

    let mut state = DashboardState::new();

    // Spawn background data tasks.
    let (tx, mut rx) = mpsc::unbounded_channel::<FeedMessage>();
    feed::spawn_rest_poller(&ctx.api_url, tx.clone());
    feed::spawn_ws_listener(&ctx.api_url, tx);

    // Main event loop: poll terminal events and feed messages.
    loop {
        // Draw the current state.
        terminal
            .draw(|f| ui::draw(f, &state))
            .ok();

        // Check for terminal input events (non-blocking, 50ms timeout).
        if ct_event::poll(Duration::from_millis(50)).unwrap_or(false) {
            if let Ok(Event::Key(key)) = ct_event::read() {
                let action = input::handle_key(&mut state, key);
                match action {
                    InputAction::Approve | InputAction::Reject => {
                        state.show_confirm_dialog = true;
                        // TODO: wire approve/reject API calls (Task 8).
                    }
                    InputAction::None => {}
                }
            }
        }

        // Drain all pending feed messages.
        while let Ok(msg) = rx.try_recv() {
            match msg {
                FeedMessage::StatusUpdate {
                    runtime,
                    agents,
                    approvals_summary,
                    pending_approvals,
                    budget,
                } => {
                    state.runtime = runtime;
                    state.agents = agents;
                    state.approvals_summary = approvals_summary;
                    state.pending_approvals = pending_approvals;
                    state.budget = budget;
                    // Clamp approval selection to valid range.
                    if !state.pending_approvals.is_empty() {
                        state.approval_selected = state
                            .approval_selected
                            .min(state.pending_approvals.len() - 1);
                    } else {
                        state.approval_selected = 0;
                    }
                }
                FeedMessage::Event(entry) => {
                    state.push_event(entry);
                }
                FeedMessage::WsDisconnected => {
                    // WS dropped — REST poller keeps going, so just note it.
                }
            }
        }

        if state.should_quit {
            break;
        }
    }

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

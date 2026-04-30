//! `aasm approvals watch` — live-updating approval request stream.

use std::io::Write;

use chrono::Utc;
use clap::Args;
use crossterm::event::{KeyCode, KeyEvent};
use crossterm::terminal;
use futures_util::StreamExt;
use tokio_tungstenite::tungstenite::Message;

use super::models::{compute_timeout_color, format_countdown, TimeoutColor};

use crate::config::ResolvedContext;
use crate::error::CliError;

use super::client;
use super::models::ApprovalResponse;

/// Type alias for the WebSocket stream used by the watch command.
pub type WsStream =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

/// Arguments for the `aasm approvals watch` subcommand.
#[derive(Debug, Args)]
pub struct WatchArgs {
    /// Enable interactive mode with keyboard shortcuts (a=approve, r=reject, q=quit).
    #[arg(long, short)]
    pub interactive: bool,
}

/// Establish a WebSocket connection to the approval events endpoint.
pub async fn connect_approval_ws(ctx: &ResolvedContext) -> Result<WsStream, CliError> {
    let url = client::build_ws_url(&ctx.api_url, "approval_required")?;
    let (ws, _response) =
        tokio_tungstenite::connect_async(&url)
            .await
            .map_err(|e| CliError::Io(std::io::Error::new(std::io::ErrorKind::ConnectionRefused, e)))?;
    Ok(ws)
}

/// Mutable state for the interactive watch mode.
///
/// Tracks the list of pending approvals and the user's current selection.
pub struct InteractiveState {
    /// Currently pending approval items.
    pub items: Vec<ApprovalResponse>,
    /// Index of the currently selected item in `items`.
    pub selected: usize,
    /// Whether the view needs to be redrawn.
    pub dirty: bool,
}

impl InteractiveState {
    /// Create a new empty interactive state.
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            selected: 0,
            dirty: true,
        }
    }

    /// Move selection up by one.
    pub fn select_prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.dirty = true;
        }
    }

    /// Move selection down by one.
    pub fn select_next(&mut self) {
        if !self.items.is_empty() && self.selected < self.items.len() - 1 {
            self.selected += 1;
            self.dirty = true;
        }
    }

    /// Return the ID of the currently selected item, if any.
    pub fn selected_id(&self) -> Option<&str> {
        self.items.get(self.selected).map(|i| i.id.as_str())
    }
}

/// Actions that can result from a keypress in interactive mode.
pub enum KeyAction {
    /// Approve the currently selected item.
    Approve,
    /// Reject the currently selected item (will prompt for reason).
    Reject,
    /// Quit the interactive watch.
    Quit,
    /// No action needed (navigation was handled internally).
    None,
}

/// Handle a keypress event in interactive mode.
///
/// Arrow keys adjust the selection. `a` triggers approve, `r` triggers reject,
/// `q` quits.
pub fn handle_keypress(key: KeyEvent, state: &mut InteractiveState) -> KeyAction {
    match key.code {
        KeyCode::Up => {
            state.select_prev();
            KeyAction::None
        }
        KeyCode::Down => {
            state.select_next();
            KeyAction::None
        }
        KeyCode::Char('a') | KeyCode::Char('A') => KeyAction::Approve,
        KeyCode::Char('r') | KeyCode::Char('R') => KeyAction::Reject,
        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => KeyAction::Quit,
        _ => KeyAction::None,
    }
}

/// Render the interactive view to stdout.
///
/// Clears the terminal and draws the approval list with the current selection
/// highlighted, plus a help bar at the bottom.
pub fn render_interactive_view(state: &InteractiveState) {
    let mut stdout = std::io::stdout();

    // Clear screen and move cursor to top-left.
    let _ = crossterm::execute!(
        stdout,
        terminal::Clear(terminal::ClearType::All),
        crossterm::cursor::MoveTo(0, 0)
    );

    println!("  aasm approvals watch (interactive)");
    println!("  [a] approve  [r] reject  [Up/Down] navigate  [q] quit");
    println!();

    if state.items.is_empty() {
        println!("  No pending approvals.");
        let _ = stdout.flush();
        return;
    }

    let now = Utc::now().timestamp();

    for (i, item) in state.items.iter().enumerate() {
        let marker = if i == state.selected { ">" } else { " " };
        let submitted_epoch = chrono::DateTime::parse_from_rfc3339(&item.created_at)
            .map(|dt| dt.timestamp())
            .unwrap_or(0);
        let remaining = (submitted_epoch + 300) - now;
        let color_code = match compute_timeout_color(remaining) {
            TimeoutColor::Red => "\x1b[31m",
            TimeoutColor::Yellow => "\x1b[33m",
            TimeoutColor::Green => "\x1b[32m",
        };
        let countdown = format_countdown(remaining);

        println!(
            "  {marker} {id}  {agent:<20} {action:<30} {color}{cd}\x1b[0m",
            id = &item.id[..8.min(item.id.len())],
            agent = item.agent_id,
            action = item.action,
            color = color_code,
            cd = countdown,
        );
    }

    let _ = stdout.flush();
}

/// Run the watch stream in non-interactive mode, printing events as they arrive.
pub async fn run_watch_stream(mut ws: WsStream) {
    println!("Watching for approval requests... (Ctrl+C to stop)");
    println!();

    while let Some(msg) = ws.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(approval) = serde_json::from_str::<ApprovalResponse>(&text) {
                    println!(
                        "  \x1b[1;33mNEW\x1b[0m  {} | agent={} | action={} | condition={}",
                        approval.id, approval.agent_id, approval.action, approval.reason
                    );
                    println!(
                        "        run: aasm approvals approve {} --reason \"...\"",
                        approval.id
                    );
                    println!();
                }
            }
            Ok(Message::Close(_)) => {
                println!("Connection closed by server.");
                break;
            }
            Err(e) => {
                eprintln!("WebSocket error: {e}");
                break;
            }
            _ => {}
        }
    }
}

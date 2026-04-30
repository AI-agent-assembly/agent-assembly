//! `aasm approvals watch` — live-updating approval request stream.

use clap::Args;
use futures_util::StreamExt;
use tokio_tungstenite::tungstenite::Message;

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

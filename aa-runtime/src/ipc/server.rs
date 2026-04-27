//! Unix domain socket IPC server.
//!
//! `IpcServer` binds to a UDS path, enforces connection limits via a semaphore,
//! and dispatches each connection to a pair of reader/writer tasks.

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::net::UnixListener;
use tokio::sync::{mpsc, Semaphore};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use crate::ipc::message::{IpcFrame, IpcResponse};

/// Configuration for the IPC server.
#[derive(Debug, Clone)]
pub struct IpcServerConfig {
    /// Absolute path to the Unix domain socket file.
    pub socket_path: PathBuf,
    /// Maximum number of concurrent SDK connections.
    pub max_connections: usize,
    /// Channel capacity for decoded inbound frames.
    pub inbound_channel_capacity: usize,
}

impl IpcServerConfig {
    /// Build an `IpcServerConfig` from a `RuntimeConfig`.
    pub fn from_runtime_config(config: &crate::config::RuntimeConfig) -> Self {
        Self {
            socket_path: PathBuf::from(format!("/tmp/aa-runtime-{}.sock", config.agent_id)),
            max_connections: config.ipc_max_connections,
            inbound_channel_capacity: 256,
        }
    }
}

/// The IPC server handle. Owns the bound `UnixListener`.
pub struct IpcServer {
    config: IpcServerConfig,
    listener: UnixListener,
}

impl IpcServer {
    /// Bind the Unix domain socket, removing any stale socket file first.
    ///
    /// Sets `0600` permissions on the socket file after binding.
    pub fn bind(config: IpcServerConfig) -> std::io::Result<Self> {
        let path = &config.socket_path;

        // Remove stale socket if it exists.
        if path.exists() {
            std::fs::remove_file(path)?;
            tracing::info!(path = %path.display(), "removed stale socket file");
        }

        let listener = UnixListener::bind(path)?;

        // Set owner-only permissions (0600).
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;

        tracing::info!(
            path = %path.display(),
            max_connections = config.max_connections,
            "IPC server bound"
        );

        Ok(Self { config, listener })
    }

    /// Run the accept loop until the cancellation token fires.
    ///
    /// Each accepted connection is handed off to a pair of reader/writer tasks
    /// registered with the provided `TaskTracker`.
    pub async fn run(
        self,
        tracker: TaskTracker,
        token: CancellationToken,
        inbound_tx: mpsc::Sender<IpcFrame>,
    ) {
        let semaphore = Arc::new(Semaphore::new(self.config.max_connections));
        let listener = self.listener;
        let socket_path = self.config.socket_path.clone();
        let inbound_channel_capacity = self.config.inbound_channel_capacity;
        let max_connections = self.config.max_connections;

        tracing::info!("IPC server accept loop started");

        loop {
            tokio::select! {
                _ = token.cancelled() => {
                    tracing::info!("IPC server shutting down — cancellation received");
                    break;
                }
                result = listener.accept() => {
                    match result {
                        Err(e) => {
                            tracing::error!(error = %e, "accept error");
                            continue;
                        }
                        Ok((stream, _addr)) => {
                            // Acquire a connection permit (non-blocking try first).
                            let permit = match Arc::clone(&semaphore).try_acquire_owned() {
                                Ok(p) => p,
                                Err(_) => {
                                    tracing::warn!(
                                        max = max_connections,
                                        "connection limit reached — dropping new connection"
                                    );
                                    drop(stream);
                                    continue;
                                }
                            };

                            let frame_tx = inbound_tx.clone();
                            let conn_token = token.child_token();

                            // Per-connection outbound channel.
                            let (resp_tx, resp_rx) =
                                mpsc::channel::<IpcResponse>(inbound_channel_capacity);

                            // Spawn connection handler tasks.
                            spawn_connection(
                                &tracker,
                                stream,
                                frame_tx,
                                resp_tx,
                                resp_rx,
                                conn_token,
                                permit,
                            );
                        }
                    }
                }
            }
        }

        // Clean up socket file on shutdown.
        if let Err(e) = std::fs::remove_file(&socket_path) {
            tracing::warn!(error = %e, "failed to remove socket file on shutdown");
        }

        tracing::info!("IPC server accept loop stopped");
    }
}

/// Spawn reader and writer tasks for a single accepted connection.
pub(super) fn spawn_connection(
    tracker: &TaskTracker,
    stream: tokio::net::UnixStream,
    frame_tx: mpsc::Sender<IpcFrame>,
    _resp_tx: mpsc::Sender<IpcResponse>,
    resp_rx: mpsc::Receiver<IpcResponse>,
    token: CancellationToken,
    permit: tokio::sync::OwnedSemaphorePermit,
) {
    let (read_half, write_half) = stream.into_split();

    // Reader task: decode frames from socket → inbound channel.
    let reader_token = token.clone();
    let reader_frame_tx = frame_tx;
    tracker.spawn(async move {
        let _permit = permit; // held until reader task completes
        run_reader(read_half, reader_frame_tx, reader_token).await;
    });

    // Writer task: outbound responses → socket.
    tracker.spawn(async move {
        run_writer(write_half, resp_rx, token).await;
    });
}

/// Reader task: reads frames from the socket and sends them to the inbound channel.
pub(super) async fn run_reader(
    mut stream: tokio::net::unix::OwnedReadHalf,
    frame_tx: mpsc::Sender<IpcFrame>,
    token: CancellationToken,
) {
    loop {
        tokio::select! {
            _ = token.cancelled() => {
                tracing::debug!("reader task cancelled");
                break;
            }
            result = super::codec::read_frame(&mut stream) => {
                match result {
                    Ok(frame) => {
                        if frame_tx.send(frame).await.is_err() {
                            tracing::debug!("inbound channel closed — reader exiting");
                            break;
                        }
                    }
                    Err(super::codec::CodecError::Io(e))
                        if e.kind() == std::io::ErrorKind::UnexpectedEof
                            || e.kind() == std::io::ErrorKind::ConnectionReset =>
                    {
                        tracing::debug!("SDK client disconnected");
                        break;
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "frame decode error — closing connection");
                        break;
                    }
                }
            }
        }
    }
    token.cancel(); // Signal the paired writer to stop.
}

/// Writer task: reads responses from the channel and writes them to the socket.
pub(super) async fn run_writer(
    mut stream: tokio::net::unix::OwnedWriteHalf,
    mut resp_rx: mpsc::Receiver<IpcResponse>,
    token: CancellationToken,
) {
    loop {
        tokio::select! {
            _ = token.cancelled() => {
                tracing::debug!("writer task cancelled");
                break;
            }
            maybe_resp = resp_rx.recv() => {
                match maybe_resp {
                    None => {
                        tracing::debug!("response channel closed — writer exiting");
                        break;
                    }
                    Some(response) => {
                        if let Err(e) = super::codec::write_response(&mut stream, response).await {
                            tracing::warn!(error = %e, "failed to write response — closing connection");
                            break;
                        }
                    }
                }
            }
        }
    }
}

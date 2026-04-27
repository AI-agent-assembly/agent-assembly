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
    pub async fn run(self, tracker: TaskTracker, token: CancellationToken, inbound_tx: mpsc::Sender<IpcFrame>) {
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
                            // NOTE: resp_tx is not yet wired to a response dispatcher — response routing
                            // is out of scope for AAASM-30 and will be implemented in a follow-on ticket.
                            // The writer task will exit immediately when resp_tx is dropped here.
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
    // _resp_tx: not yet used — response routing is out of scope for AAASM-30.
    // Will be wired to the governance dispatcher in a follow-on ticket.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::codec::{TAG_EVENT_REPORT, TAG_HEARTBEAT, TAG_POLICY_QUERY};
    use crate::ipc::message::IpcFrame;
    use aa_proto::assembly::audit::v1::AuditEvent;
    use aa_proto::assembly::policy::v1::CheckActionRequest;
    use prost::Message;
    use std::time::Duration;
    use tokio::net::UnixStream;
    use tokio::sync::mpsc;

    /// Build a temporary socket path unique per test to avoid collisions.
    fn temp_socket_path(name: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(format!("/tmp/aa-runtime-test-{name}.sock"))
    }

    /// Helper: connect a mock SDK client to the server socket, retrying briefly.
    async fn connect_client(path: &std::path::Path) -> UnixStream {
        for _ in 0..20 {
            if let Ok(stream) = UnixStream::connect(path).await {
                return stream;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("could not connect to test IPC server at {}", path.display());
    }

    /// Start a test IpcServer and return the inbound frame receiver.
    async fn start_server(socket_path: std::path::PathBuf, token: CancellationToken) -> mpsc::Receiver<IpcFrame> {
        let config = IpcServerConfig {
            socket_path,
            max_connections: 64,
            inbound_channel_capacity: 16,
        };
        let server = IpcServer::bind(config).expect("bind failed");
        let (tx, rx) = mpsc::channel(16);
        let tracker = TaskTracker::new();
        let tracker_clone = tracker.clone();
        tracker.spawn(async move {
            server.run(tracker_clone, token, tx).await;
        });
        rx
    }

    /// Write a raw inbound frame (tag + varint len + payload) to the socket.
    async fn write_raw_frame(stream: &mut tokio::net::unix::OwnedWriteHalf, tag: u8, payload: &[u8]) {
        use tokio::io::AsyncWriteExt;
        stream.write_u8(tag).await.unwrap();
        // Write varint length
        let mut len = payload.len() as u64;
        loop {
            let byte = (len & 0x7F) as u8;
            len >>= 7;
            if len == 0 {
                stream.write_u8(byte).await.unwrap();
                break;
            } else {
                stream.write_u8(byte | 0x80).await.unwrap();
            }
        }
        stream.write_all(payload).await.unwrap();
        stream.flush().await.unwrap();
    }

    #[tokio::test]
    async fn heartbeat_frame_arrives_on_inbound_channel() {
        let socket_path = temp_socket_path("heartbeat");
        let token = CancellationToken::new();
        let mut rx = start_server(socket_path.clone(), token.clone()).await;

        let client = connect_client(&socket_path).await;
        let (_, mut write_half) = client.into_split();

        // Heartbeat has tag only, no payload or length field.
        use tokio::io::AsyncWriteExt;
        write_half.write_u8(TAG_HEARTBEAT).await.unwrap();
        write_half.flush().await.unwrap();

        let frame = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("timed out waiting for frame")
            .expect("channel closed");

        assert!(matches!(frame, IpcFrame::Heartbeat));
        token.cancel();
    }

    #[tokio::test]
    async fn policy_query_arrives_decoded_on_inbound_channel() {
        let socket_path = temp_socket_path("policy-query");
        let token = CancellationToken::new();
        let mut rx = start_server(socket_path.clone(), token.clone()).await;

        let client = connect_client(&socket_path).await;
        let (_, mut write_half) = client.into_split();

        let request = CheckActionRequest {
            trace_id: "trace-xyz".to_string(),
            ..Default::default()
        };
        let payload = request.encode_to_vec();
        write_raw_frame(&mut write_half, TAG_POLICY_QUERY, &payload).await;

        let frame = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("timed out")
            .expect("channel closed");

        match frame {
            IpcFrame::PolicyQuery(decoded) => assert_eq!(decoded.trace_id, "trace-xyz"),
            other => panic!("expected PolicyQuery, got {other:?}"),
        }
        token.cancel();
    }

    #[tokio::test]
    async fn event_report_arrives_decoded_on_inbound_channel() {
        let socket_path = temp_socket_path("event-report");
        let token = CancellationToken::new();
        let mut rx = start_server(socket_path.clone(), token.clone()).await;

        let client = connect_client(&socket_path).await;
        let (_, mut write_half) = client.into_split();

        let event = AuditEvent {
            event_id: "evt-456".to_string(),
            ..Default::default()
        };
        let payload = event.encode_to_vec();
        write_raw_frame(&mut write_half, TAG_EVENT_REPORT, &payload).await;

        let frame = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("timed out")
            .expect("channel closed");

        match frame {
            IpcFrame::EventReport(decoded) => assert_eq!(decoded.event_id, "evt-456"),
            other => panic!("expected EventReport, got {other:?}"),
        }
        token.cancel();
    }

    #[tokio::test]
    async fn concurrent_connections_up_to_limit() {
        let socket_path = temp_socket_path("concurrent");
        let token = CancellationToken::new();
        let _rx = start_server(socket_path.clone(), token.clone()).await;

        const CONN_COUNT: usize = 5;
        let mut clients = Vec::new();
        for _ in 0..CONN_COUNT {
            clients.push(connect_client(&socket_path).await);
        }

        // All connections should succeed (well below max of 64).
        assert_eq!(clients.len(), CONN_COUNT);
        token.cancel();
    }

    /// Round-trip latency test. Marked #[ignore] — run explicitly only.
    #[tokio::test]
    #[ignore]
    async fn round_trip_latency_under_1ms() {
        let socket_path = temp_socket_path("latency");
        let token = CancellationToken::new();
        let mut rx = start_server(socket_path.clone(), token.clone()).await;

        let client = connect_client(&socket_path).await;
        let (_, mut write_half) = client.into_split();

        const ITERATIONS: u32 = 1000;
        let start = std::time::Instant::now();

        for _ in 0..ITERATIONS {
            use tokio::io::AsyncWriteExt;
            write_half.write_u8(TAG_HEARTBEAT).await.unwrap();
            write_half.flush().await.unwrap();
            tokio::time::timeout(Duration::from_millis(100), rx.recv())
                .await
                .expect("timed out")
                .expect("channel closed");
        }

        let elapsed = start.elapsed();
        let avg_us = elapsed.as_micros() / ITERATIONS as u128;
        println!("Average round-trip: {avg_us} µs");

        assert!(avg_us < 1000, "average round-trip {avg_us} µs exceeded 1ms threshold");

        token.cancel();
    }
}

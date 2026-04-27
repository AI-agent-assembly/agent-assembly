//! Unix domain socket IPC server for local SDK-to-runtime communication.

pub mod codec;
pub mod message;
pub mod server;

pub use message::IpcFrame;
pub use server::IpcServer;

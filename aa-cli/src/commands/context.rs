//! `aasm context` — manage named API contexts.

use std::process::ExitCode;

use clap::{Args, Subcommand};

use crate::config;

/// Arguments for the `aasm context` subcommand group.
#[derive(Args)]
pub struct ContextArgs {
    #[command(subcommand)]
    pub command: ContextCommands,
}

/// Available context subcommands.
#[derive(Subcommand)]
pub enum ContextCommands {
    /// List all configured contexts.
    List,
    /// Set or create a named context.
    Set(SetArgs),
    /// Switch the default context.
    Use(UseArgs),
}

/// Arguments for `aasm context set`.
#[derive(Args)]
pub struct SetArgs {
    /// Name of the context to create or update.
    pub name: String,
    /// API URL for this context.
    #[arg(long)]
    pub api_url: String,
    /// API key for this context (optional).
    #[arg(long)]
    pub api_key: Option<String>,
}

/// Arguments for `aasm context use`.
#[derive(Args)]
pub struct UseArgs {
    /// Name of the context to set as default.
    pub name: String,
}

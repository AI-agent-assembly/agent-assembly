//! Event notification subsystem — webhook delivery for governance events.
//!
//! Subscribes to the runtime's `PipelineEvent` broadcast and the budget
//! tracker's `BudgetAlert` broadcast, converts relevant events into
//! [`EnvelopedEvent`](aa_proto::assembly::event::v1::EnvelopedEvent) envelopes,
//! and delivers them as JSON via HTTP POST to a configured webhook URL.

pub mod publisher;
pub mod webhook;

use std::fmt;

/// Errors that can occur during event publishing.
#[derive(Debug)]
pub enum PublishError {
    /// The HTTP request to the webhook endpoint failed.
    Http(reqwest::Error),
    /// JSON serialization of the event envelope failed.
    Serialization(String),
}

impl fmt::Display for PublishError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PublishError::Http(e) => write!(f, "webhook HTTP error: {e}"),
            PublishError::Serialization(msg) => write!(f, "event serialization error: {msg}"),
        }
    }
}

impl std::error::Error for PublishError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PublishError::Http(e) => Some(e),
            PublishError::Serialization(_) => None,
        }
    }
}

impl From<reqwest::Error> for PublishError {
    fn from(e: reqwest::Error) -> Self {
        PublishError::Http(e)
    }
}

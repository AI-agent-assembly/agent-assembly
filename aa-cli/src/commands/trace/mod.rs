//! `aasm trace` — session trace visualization.

use clap::ValueEnum;

pub mod models;

/// Visualization format for trace output.
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum TraceFormat {
    /// Indented tree with box-drawing characters (default).
    #[default]
    Tree,
    /// Horizontal ASCII timeline with duration bars.
    Timeline,
}

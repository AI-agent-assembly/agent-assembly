//! `aasm trace` — session trace visualization.

use std::process::ExitCode;

use clap::{Args, ValueEnum};

use crate::config::ResolvedContext;
use crate::output::OutputFormat;

pub mod client;
pub mod models;
pub mod timeline;
pub mod tree;

/// Visualization format for trace output.
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum TraceFormat {
    /// Indented tree with box-drawing characters (default).
    #[default]
    Tree,
    /// Horizontal ASCII timeline with duration bars.
    Timeline,
}

/// Arguments for the `aasm trace` subcommand.
#[derive(Debug, Args)]
pub struct TraceArgs {
    /// Session ID to retrieve the trace for.
    pub session_id: String,

    /// Visualization format.
    #[arg(long, value_enum, default_value_t = TraceFormat::Tree)]
    pub format: TraceFormat,
}

/// Execute the `aasm trace` subcommand.
pub fn dispatch(args: TraceArgs, ctx: &ResolvedContext, output: OutputFormat) -> ExitCode {
    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    let trace = match rt.block_on(client::fetch_trace(ctx, &args.session_id)) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error fetching trace: {e}");
            return ExitCode::FAILURE;
        }
    };

    match output {
        OutputFormat::Json => {
            match serde_json::to_string_pretty(&trace) {
                Ok(json) => println!("{json}"),
                Err(e) => {
                    eprintln!("error serializing trace: {e}");
                    return ExitCode::FAILURE;
                }
            }
        }
        OutputFormat::Yaml => {
            match serde_yaml::to_string(&trace) {
                Ok(yaml) => print!("{yaml}"),
                Err(e) => {
                    eprintln!("error serializing trace: {e}");
                    return ExitCode::FAILURE;
                }
            }
        }
        OutputFormat::Table => match args.format {
            TraceFormat::Tree => {
                print!("{}", tree::render_tree(&trace));
            }
            TraceFormat::Timeline => {
                // TODO: wire timeline format
                println!("{trace:?}");
            }
        },
    }

    ExitCode::SUCCESS
}

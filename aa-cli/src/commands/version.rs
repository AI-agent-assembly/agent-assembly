//! `aasm version` — display CLI and runtime version information.

use std::process::ExitCode;

use crate::config::ResolvedContext;

/// Print CLI version and, if reachable, the gateway runtime version.
pub fn run(ctx: &ResolvedContext) -> ExitCode {
    println!("aasm {}", env!("CARGO_PKG_VERSION"));
    println!("api:  {}", ctx.api_url);

    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    rt.block_on(async {
        let client = reqwest::Client::new();
        let url = format!("{}/api/v1/health", ctx.api_url);
        match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                println!("gateway: reachable");
            }
            Ok(resp) => {
                println!("gateway: responded with {}", resp.status());
            }
            Err(_) => {
                println!("gateway: unreachable");
            }
        }
    });
    ExitCode::SUCCESS
}

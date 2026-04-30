//! Shared HTTP client for communicating with the Agent Assembly gateway.

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::config::ResolvedContext;
use crate::error::CliError;

/// Build a [`reqwest::Client`] with default settings.
pub fn build_client() -> reqwest::Client {
    reqwest::Client::new()
}

/// Perform a GET request to the gateway and deserialize the JSON response.
pub async fn get_json<T: DeserializeOwned>(ctx: &ResolvedContext, path: &str) -> Result<T, CliError> {
    let url = format!("{}{path}", ctx.api_url);
    let client = build_client();

    let mut req = client.get(&url);
    if let Some(ref key) = ctx.api_key {
        req = req.bearer_auth(key);
    }

    let resp = req.send().await?.error_for_status()?;
    let body = resp.json::<T>().await?;
    Ok(body)
}

/// Perform a POST request to the gateway with a JSON body and deserialize the response.
pub async fn post_json<B: Serialize, T: DeserializeOwned>(
    ctx: &ResolvedContext,
    path: &str,
    body: &B,
) -> Result<T, CliError> {
    let url = format!("{}{path}", ctx.api_url);
    let client = build_client();

    let mut req = client.post(&url).json(body);
    if let Some(ref key) = ctx.api_key {
        req = req.bearer_auth(key);
    }

    let resp = req.send().await?.error_for_status()?;
    let result = resp.json::<T>().await?;
    Ok(result)
}

/// Perform a DELETE request to the gateway.
pub async fn delete(ctx: &ResolvedContext, path: &str) -> Result<(), CliError> {
    let url = format!("{}{path}", ctx.api_url);
    let client = build_client();

    let mut req = client.delete(&url);
    if let Some(ref key) = ctx.api_key {
        req = req.bearer_auth(key);
    }

    req.send().await?.error_for_status()?;
    Ok(())
}

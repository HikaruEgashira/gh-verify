use anyhow::{Context, Result, bail};
use std::process::Command;

pub struct Config {
    pub token: String,
    pub repo: String,
    pub host: String,
}

impl Config {
    pub fn load() -> Result<Self> {
        let token = resolve_token()?;
        let repo = std::env::var("GH_REPO").unwrap_or_default();
        let host = std::env::var("GH_HOST").unwrap_or_else(|_| "api.github.com".to_string());
        validate_host(&host)?;
        Ok(Self { token, repo, host })
    }
}

fn validate_host(host: &str) -> Result<()> {
    if host.is_empty() {
        bail!("invalid host: empty");
    }
    if host.starts_with("localhost") {
        bail!("invalid host: localhost not allowed");
    }
    if host.as_bytes()[0].is_ascii_digit() {
        bail!("invalid host: IP addresses not allowed");
    }
    if !host.contains('.') {
        bail!("invalid host: must contain a dot");
    }
    Ok(())
}

fn resolve_token() -> Result<String> {
    if let Ok(token) = std::env::var("GH_TOKEN") {
        return Ok(token);
    }
    if let Ok(token) = std::env::var("GH_ENTERPRISE_TOKEN") {
        return Ok(token);
    }
    // Fallback: run `gh auth token`
    let output = Command::new("gh")
        .args(["auth", "token"])
        .output()
        .context("failed to run `gh auth token`")?;
    let token = String::from_utf8(output.stdout)
        .context("invalid UTF-8 in gh auth token output")?
        .trim()
        .to_string();
    if token.is_empty() {
        bail!("no GitHub token found. Set GH_TOKEN or run `gh auth login`");
    }
    Ok(token)
}

use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use reqwest::header::{ACCEPT, AUTHORIZATION, HeaderMap, HeaderValue, USER_AGENT};

use crate::config::Config;

const MAX_BODY_SIZE: usize = 10 * 1024 * 1024; // 10MB

pub struct GitHubClient {
    client: Client,
    base_url: String,
}

impl GitHubClient {
    pub fn new(cfg: &Config) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", cfg.token)).context("invalid token")?,
        );
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/vnd.github.v3+json"),
        );
        headers.insert(
            "X-GitHub-Api-Version",
            HeaderValue::from_static("2022-11-28"),
        );
        headers.insert(USER_AGENT, HeaderValue::from_static("gh-verify/0.2.0"));

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .context("failed to create HTTP client")?;

        Ok(Self {
            client,
            base_url: format!("https://{}", cfg.host),
        })
    }

    /// GET request returning body as string.
    pub fn get(&self, path: &str) -> Result<String> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .client
            .get(&url)
            .send()
            .context("HTTP request failed")?;

        let status = resp.status();
        if !status.is_success() {
            bail!(
                "GitHub API error: {} {}",
                status.as_u16(),
                status.canonical_reason().unwrap_or("Unknown")
            );
        }

        let body = resp.text().context("failed to read response body")?;
        if body.len() > MAX_BODY_SIZE {
            bail!("response too large: {} bytes", body.len());
        }
        Ok(body)
    }

    /// GET request with pagination support. Returns (body, next_url).
    pub fn get_with_link(&self, path: &str) -> Result<(String, Option<String>)> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .client
            .get(&url)
            .send()
            .context("HTTP request failed")?;

        let status = resp.status();
        if !status.is_success() {
            bail!(
                "GitHub API error: {} {}",
                status.as_u16(),
                status.canonical_reason().unwrap_or("Unknown")
            );
        }

        let next_url = resp
            .headers()
            .get("link")
            .and_then(|v| v.to_str().ok())
            .and_then(|link| parse_link_next(link, &self.base_url));

        let body = resp.text().context("failed to read response body")?;
        if body.len() > MAX_BODY_SIZE {
            bail!("response too large: {} bytes", body.len());
        }
        Ok((body, next_url))
    }
}

/// Extract the path for rel="next" from a Link header.
fn parse_link_next(link_header: &str, base_prefix: &str) -> Option<String> {
    for part in link_header.split(',') {
        let part = part.trim();
        if !part.contains("rel=\"next\"") {
            continue;
        }
        let lt = part.find('<')?;
        let gt = part.find('>')?;
        let url = &part[lt + 1..gt];
        if let Some(path) = url.strip_prefix(base_prefix) {
            return Some(path.to_string());
        }
        return Some(url.to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_link_next_extracts_path() {
        let header = r#"<https://api.github.com/repos/o/r/pulls/1/files?page=2>; rel="next", <https://api.github.com/repos/o/r/pulls/1/files?page=5>; rel="last""#;
        let result = parse_link_next(header, "https://api.github.com");
        assert_eq!(result, Some("/repos/o/r/pulls/1/files?page=2".to_string()));
    }

    #[test]
    fn parse_link_next_returns_none_without_next() {
        let header = r#"<https://api.github.com/repos/o/r/pulls/1/files?page=5>; rel="last""#;
        let result = parse_link_next(header, "https://api.github.com");
        assert!(result.is_none());
    }
}

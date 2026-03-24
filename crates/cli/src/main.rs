use std::process;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use libverify_github::range::{
    detect_latest_release_tag, parse_range, parse_release_arg, resolve_pr_numbers,
};
use libverify_github::verify::exit_if_assessment_fails;
use libverify_github::{
    GitHubClient, GitHubConfig, verify_pr, verify_pr_batch, verify_release, verify_repo,
};

mod output;

const VERSION: &str = env!("GH_VERIFY_VERSION");

#[derive(Parser)]
#[command(name = "gh-verify", version = VERSION, about = "GitHub SDLC health checker")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Verify a pull request
    Pr {
        /// PR number or range (#N..#M, SHA..SHA, TAG..TAG, DATE..DATE)
        arg: Option<String>,
        /// Output format (human, json, or sarif)
        #[arg(long, default_value = "human")]
        format: String,
        /// Repository in OWNER/REPO format
        #[arg(long)]
        repo: Option<String>,
        /// Policy: preset name (default, oss, aiops, soc1, soc2, slsa-l1..l4) or .rego file path
        #[arg(long)]
        policy: Option<String>,
        /// Include raw collected evidence in output
        #[arg(long)]
        with_evidence: bool,
        /// Only show failing controls in output
        #[arg(long)]
        only_failures: bool,
    },
    /// Verify repository dependency signatures
    Repo {
        /// Git reference (branch, tag, or SHA). Defaults to HEAD.
        #[arg(long, default_value = "HEAD")]
        r#ref: String,
        /// Output format (human, json, or sarif)
        #[arg(long, default_value = "human")]
        format: String,
        /// Repository in OWNER/REPO format
        #[arg(long)]
        repo: Option<String>,
        /// Policy: preset name (default, oss, aiops, soc1, soc2, slsa-l1..l4) or .rego file path
        #[arg(long)]
        policy: Option<String>,
        /// Include raw collected evidence in output
        #[arg(long)]
        with_evidence: bool,
        /// Only show failing controls in output
        #[arg(long)]
        only_failures: bool,
    },
    /// Convert JSON output to another format (reads from stdin)
    Fmt {
        /// Output format (human, json, or sarif)
        #[arg(long, default_value = "human")]
        format: String,
        /// Only show failing controls in output
        #[arg(long)]
        only_failures: bool,
        /// Interpret input as batch output
        #[arg(long)]
        batch: bool,
    },
    /// Verify release integrity
    Release {
        /// Tag or BASE..HEAD range (omit to use latest release)
        arg: Option<String>,
        /// Output format (human, json, or sarif)
        #[arg(long, default_value = "human")]
        format: String,
        /// Repository in OWNER/REPO format
        #[arg(long)]
        repo: Option<String>,
        /// Policy: preset name (default, oss, aiops, soc1, soc2, slsa-l1..l4) or .rego file path
        #[arg(long)]
        policy: Option<String>,
        /// Include raw collected evidence in output
        #[arg(long)]
        with_evidence: bool,
        /// Only show failing controls in output
        #[arg(long)]
        only_failures: bool,
    },
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e:#}");
        process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Pr {
            arg,
            format,
            repo: repo_override,
            policy,
            with_evidence,
            only_failures,
        } => {
            let opts = output::OutputOptions {
                format: output::parse_format(&format)?,
                only_failures,
            };
            let cfg = GitHubConfig::load()?;
            let (owner, repo_name) = resolve_repo(&cfg, repo_override.as_deref())?;
            let client = GitHubClient::new(&cfg)?;

            match arg.as_deref().and_then(parse_range) {
                Some(spec) => {
                    let pr_numbers = resolve_pr_numbers(&spec, &client, &owner, &repo_name)?;
                    if pr_numbers.is_empty() {
                        println!("No merged PRs found for the given range");
                        return Ok(());
                    }
                    eprintln!("Found {} PRs to verify", pr_numbers.len());
                    let batch = verify_pr_batch(
                        &client,
                        &owner,
                        &repo_name,
                        &pr_numbers,
                        policy.as_deref(),
                        with_evidence,
                    )?;
                    output::print_batch(&opts, &batch)?;
                    if batch.total_fail > 0 {
                        process::exit(1);
                    }
                }
                None => {
                    let pr_number: u32 = match arg {
                        Some(a) => a.parse().context("invalid PR number")?,
                        None => detect_pr_number()?,
                    };
                    let result = verify_pr(
                        &client,
                        &owner,
                        &repo_name,
                        pr_number,
                        policy.as_deref(),
                        with_evidence,
                    )?;
                    output::print(&opts, &result)?;
                    exit_if_assessment_fails(&result);
                }
            }
        }
        Commands::Fmt {
            format,
            only_failures,
            batch,
        } => {
            let opts = output::OutputOptions {
                format: output::parse_format(&format)?,
                only_failures,
            };
            let input = std::io::read_to_string(std::io::stdin())
                .context("failed to read JSON from stdin")?;
            if batch {
                let batch_report: libverify_core::assessment::BatchReport =
                    serde_json::from_str(&input).context("invalid batch JSON on stdin")?;
                output::print_batch(&opts, &batch_report)?;
            } else {
                let result: libverify_core::assessment::VerificationResult =
                    serde_json::from_str(&input).context("invalid JSON on stdin")?;
                output::print(&opts, &result)?;
                exit_if_assessment_fails(&result);
            }
        }
        Commands::Repo {
            r#ref: reference,
            format,
            repo: repo_override,
            policy,
            with_evidence,
            only_failures,
        } => {
            let opts = output::OutputOptions {
                format: output::parse_format(&format)?,
                only_failures,
            };
            let cfg = GitHubConfig::load()?;
            let (owner, repo_name) = resolve_repo(&cfg, repo_override.as_deref())?;
            let client = GitHubClient::new(&cfg)?;

            eprintln!("Checking dependency signatures at ref: {reference}");

            let result = verify_repo(
                &client,
                &owner,
                &repo_name,
                &reference,
                policy.as_deref(),
                with_evidence,
            )?;
            output::print(&opts, &result)?;
            exit_if_assessment_fails(&result);
        }
        Commands::Release {
            arg,
            format,
            repo: repo_override,
            policy,
            with_evidence,
            only_failures,
        } => {
            let opts = output::OutputOptions {
                format: output::parse_format(&format)?,
                only_failures,
            };
            let cfg = GitHubConfig::load()?;
            let (owner, repo_name) = resolve_repo(&cfg, repo_override.as_deref())?;
            let client = GitHubClient::new(&cfg)?;

            let release_arg = match arg {
                Some(a) => a,
                None => detect_latest_release_tag(&client, &owner, &repo_name)
                    .context("could not detect latest release. Pass a tag explicitly")?,
            };
            let (base_tag, head_tag) =
                parse_release_arg(&release_arg, &client, &owner, &repo_name)?;

            println!("Checking release: {base_tag}..{head_tag}");

            let result = verify_release(
                &client,
                &owner,
                &repo_name,
                &base_tag,
                &head_tag,
                policy.as_deref(),
                with_evidence,
            )?;
            output::print(&opts, &result)?;
            exit_if_assessment_fails(&result);
        }
    }

    Ok(())
}

fn resolve_repo(cfg: &GitHubConfig, override_repo: Option<&str>) -> Result<(String, String)> {
    let repo_str: String = match override_repo {
        Some(s) if !s.is_empty() => s.to_string(),
        _ if !cfg.repo.is_empty() => cfg.repo.clone(),
        _ => detect_repo_from_git_remote()?,
    };
    let slash_idx = repo_str
        .find('/')
        .context("could not resolve repo. Use --repo OWNER/REPO or set GH_REPO env var")?;
    let owner = repo_str[..slash_idx].to_string();
    let repo_name = repo_str[slash_idx + 1..].to_string();
    Ok((owner, repo_name))
}

fn detect_repo_from_git_remote() -> Result<String> {
    let output = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .context("failed to run 'git remote get-url origin' — is git installed?")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git remote get-url origin failed: {}", stderr.trim());
    }
    let url = String::from_utf8(output.stdout)
        .context("git remote URL contains invalid UTF-8")?
        .trim()
        .to_string();
    parse_github_remote_url(&url)
        .with_context(|| format!("could not parse GitHub repo from remote URL: {url}"))
}

fn parse_github_remote_url(url: &str) -> Option<String> {
    // Collect candidate hosts: GH_HOST > GITHUB_SERVER_URL > github.com fallback
    let mut hosts: Vec<String> = Vec::new();
    if let Ok(h) = std::env::var("GH_HOST") {
        let h = h.trim().to_string();
        if !h.is_empty() {
            hosts.push(h);
        }
    }
    if let Ok(server_url) = std::env::var("GITHUB_SERVER_URL") {
        // Strip protocol to get bare hostname (e.g. https://github.example.com -> github.example.com)
        let bare = server_url
            .trim()
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_end_matches('/')
            .to_string();
        if !bare.is_empty() && !hosts.contains(&bare) {
            hosts.push(bare);
        }
    }
    // Always include github.com as the final fallback
    let github_com = "github.com".to_string();
    if !hosts.contains(&github_com) {
        hosts.push(github_com);
    }

    // SSH: git@<host>:OWNER/REPO.git
    for host in &hosts {
        let prefix = format!("git@{}:", host);
        if let Some(path) = url.strip_prefix(prefix.as_str()) {
            let path = path.trim_end_matches(".git");
            if path.matches('/').count() == 1 && !path.starts_with('/') {
                return Some(path.to_string());
            }
        }
    }

    // HTTPS/HTTP: https://<host>/OWNER/REPO.git
    let url = url.trim_end_matches(".git");
    for host in &hosts {
        let https_prefix = format!("https://{}/", host);
        let http_prefix = format!("http://{}/", host);
        let path = url
            .strip_prefix(https_prefix.as_str())
            .or_else(|| url.strip_prefix(http_prefix.as_str()));
        if let Some(path) = path {
            if path.matches('/').count() == 1 && !path.starts_with('/') {
                return Some(path.to_string());
            }
        }
    }

    None
}

fn detect_pr_number() -> Result<u32> {
    let output = std::process::Command::new("gh")
        .args(["pr", "view", "--json", "number", "--jq", ".number"])
        .output()
        .context("failed to run 'gh pr view' — is the GitHub CLI installed?")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "gh pr view failed (no open PR for current branch?): {}",
            stderr.trim()
        );
    }
    let stdout = String::from_utf8(output.stdout)
        .context("gh pr view returned invalid UTF-8")?;
    stdout
        .trim()
        .parse::<u32>()
        .with_context(|| format!("gh pr view returned unexpected output: {:?}", stdout.trim()))
}

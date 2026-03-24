use std::process;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use libverify_github::range::{detect_latest_release_tag, parse_range, parse_release_arg, resolve_pr_numbers};
use libverify_github::verify::exit_if_assessment_fails;
use libverify_github::{GitHubClient, GitHubConfig, verify_pr, verify_pr_batch, verify_release};

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
                        None => detect_pr_number().context(
                            "could not detect PR for current branch. Pass a PR number explicitly",
                        )?,
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
        _ => detect_repo_from_git_remote()
            .context("could not resolve repo. Use --repo OWNER/REPO, set GH_REPO, or run from a git repo with a GitHub remote")?,
    };
    let slash_idx = repo_str
        .find('/')
        .context("could not resolve repo. Use --repo OWNER/REPO or set GH_REPO env var")?;
    let owner = repo_str[..slash_idx].to_string();
    let repo_name = repo_str[slash_idx + 1..].to_string();
    Ok((owner, repo_name))
}

fn detect_repo_from_git_remote() -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let url = String::from_utf8(output.stdout).ok()?.trim().to_string();
    parse_github_remote_url(&url)
}

fn parse_github_remote_url(url: &str) -> Option<String> {
    // SSH: git@github.com:OWNER/REPO.git
    if let Some(path) = url.strip_prefix("git@github.com:") {
        return Some(path.trim_end_matches(".git").to_string());
    }
    // HTTPS: https://github.com/OWNER/REPO.git
    let url = url.trim_end_matches(".git");
    let path = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("http://github.com/"))?;
    // Ensure it has exactly owner/repo
    if path.matches('/').count() == 1 && !path.starts_with('/') {
        Some(path.to_string())
    } else {
        None
    }
}

fn detect_pr_number() -> Option<u32> {
    let output = std::process::Command::new("gh")
        .args(["pr", "view", "--json", "number", "--jq", ".number"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()?.trim().parse().ok()
}

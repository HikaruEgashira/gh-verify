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

#[derive(clap::Args)]
struct CommonOpts {
    /// Output format
    #[arg(long, default_value_t = output::Format::Human)]
    format: output::Format,
    /// Repository in OWNER/REPO format
    #[arg(long)]
    repo: Option<String>,
    /// Policy: preset name (default, oss, aiops, soc1, soc2, slsa-l1..l4) or .rego file path
    #[arg(long)]
    policy: Option<String>,
    /// Include raw collected evidence in output (only affects json/sarif formats)
    #[arg(long)]
    with_evidence: bool,
    /// Only show failing controls in output
    #[arg(long)]
    only_failures: bool,
    /// Exclude specific controls from results (comma-separated control IDs)
    #[arg(long, value_delimiter = ',')]
    exclude: Vec<String>,
}

#[derive(Parser)]
#[command(name = "gh-verify", version = VERSION,
    about = "GitHub SDLC health checker",
    long_about = "Verify pull requests, releases, and repositories against SDLC compliance controls.\nChecks include SLSA source/build integrity, SOC2 CC7/CC8, dependency signatures, and more.",
    after_help = "Examples:\n  gh verify pr 42                    Verify PR #42 in current repo\n  gh verify pr '#100..#200'           Batch verify a PR range\n  gh verify release                   Verify the latest release\n  gh verify repo --ref main           Check dependency signatures\n  gh verify pr 42 --policy oss        Use OSS-friendly policy\n  gh verify pr 42 --format sarif      Output SARIF for code scanning"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Verify a pull request
    #[command(
        after_help = "Examples:\n  gh verify pr 42                         Single PR\n  gh verify pr '#100..#200' --policy oss   All merged PRs in range\n  gh verify pr 42 --format json | jq .     JSON output\n\nRanges verify all merged PRs within the specified bounds."
    )]
    Pr {
        /// PR number, or range to batch-verify all merged PRs: #N..#M, SHA..SHA, TAG..TAG, DATE..DATE
        #[arg(value_name = "PR")]
        arg: Option<String>,
        #[command(flatten)]
        opts: CommonOpts,
    },
    /// Verify repository security posture
    #[command(
        after_help = "Examples:\n  gh verify repo\n  gh verify repo --ref main --policy soc2\n  gh verify repo --format sarif"
    )]
    Repo {
        /// Git reference (branch, tag, or SHA). Defaults to HEAD.
        #[arg(long, default_value = "HEAD")]
        r#ref: String,
        #[command(flatten)]
        opts: CommonOpts,
    },
    /// Convert JSON output to another format (reads from stdin)
    #[command(
        after_help = "Examples:\n  gh verify pr 42 --format json | gh verify fmt --format sarif\n  gh verify pr '#1..#10' --format json | gh verify fmt --batch"
    )]
    Fmt {
        /// Output format
        #[arg(long, default_value_t = output::Format::Human)]
        format: output::Format,
        /// Only show failing controls in output
        #[arg(long)]
        only_failures: bool,
        /// Interpret input as batch output
        #[arg(long)]
        batch: bool,
    },
    /// Verify release integrity
    #[command(
        after_help = "Examples:\n  gh verify release\n  gh verify release v1.0.0\n  gh verify release v1.0.0..v2.0.0"
    )]
    Release {
        /// Tag or BASE..HEAD range (omit to use latest release)
        #[arg(value_name = "TAG")]
        arg: Option<String>,
        #[command(flatten)]
        opts: CommonOpts,
    },
    /// List available controls and their descriptions
    Controls,
    /// List available policy presets and their descriptions
    Policies,
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
        Commands::Pr { arg, opts } => {
            let out_opts = output::OutputOptions {
                format: opts.format,
                only_failures: opts.only_failures,
            };
            let cfg = GitHubConfig::load()?;
            let (owner, repo_name) = resolve_repo(&cfg, opts.repo.as_deref())?;
            check_repo_exists(&owner, &repo_name)?;
            let client = GitHubClient::new(&cfg)?;

            match arg.as_deref().and_then(parse_range) {
                Some(spec) => {
                    let pr_numbers = resolve_pr_numbers(&spec, &client, &owner, &repo_name)?;
                    if pr_numbers.is_empty() {
                        println!("No merged PRs found for the given range");
                        return Ok(());
                    }
                    eprintln!("Found {} PRs to verify", pr_numbers.len());
                    let mut batch = verify_pr_batch(
                        &client,
                        &owner,
                        &repo_name,
                        &pr_numbers,
                        opts.policy.as_deref(),
                        opts.with_evidence,
                    )?;
                    apply_batch_exclusions(&mut batch, &opts.exclude);
                    output::print_batch(&out_opts, &batch)?;
                    if batch.total_fail > 0 {
                        process::exit(1);
                    }
                }
                None => {
                    let pr_number: u32 = match arg {
                        Some(ref a) => a.parse().with_context(|| {
                            format!(
                                "'{a}' is not a valid PR number. Expected a number like 42 or a range like '#1..#5'"
                            )
                        })?,
                        None => detect_pr_number()?,
                    };
                    let mut result = verify_pr(
                        &client,
                        &owner,
                        &repo_name,
                        pr_number,
                        opts.policy.as_deref(),
                        opts.with_evidence,
                    )?;
                    apply_exclusions(&mut result, &opts.exclude);
                    output::print(&out_opts, &result)?;
                    exit_if_assessment_fails(&result);
                }
            }
        }
        Commands::Fmt {
            format,
            only_failures,
            batch,
        } => {
            let out_opts = output::OutputOptions {
                format,
                only_failures,
            };
            let input = std::io::read_to_string(std::io::stdin())
                .context("failed to read JSON from stdin")?;
            let input = input.trim();
            if input.is_empty() {
                anyhow::bail!(
                    "no input on stdin. Pipe JSON from another command, e.g.:\n  gh verify pr 42 --format json | gh verify fmt --format sarif"
                );
            }
            if batch {
                let batch_report: libverify_core::assessment::BatchReport =
                    serde_json::from_str(input).context("invalid batch JSON on stdin")?;
                output::print_batch(&out_opts, &batch_report)?;
            } else {
                let result: libverify_core::assessment::VerificationResult =
                    serde_json::from_str(input).context("invalid JSON on stdin")?;
                output::print(&out_opts, &result)?;
                exit_if_assessment_fails(&result);
            }
        }
        Commands::Repo {
            r#ref: reference,
            opts,
        } => {
            let out_opts = output::OutputOptions {
                format: opts.format,
                only_failures: opts.only_failures,
            };
            let cfg = GitHubConfig::load()?;
            let (owner, repo_name) = resolve_repo(&cfg, opts.repo.as_deref())?;
            check_repo_exists(&owner, &repo_name)?;
            let client = GitHubClient::new(&cfg)?;

            eprintln!("Checking security posture at ref: {reference}");

            let mut result = verify_repo(
                &client,
                &owner,
                &repo_name,
                &reference,
                opts.policy.as_deref(),
                opts.with_evidence,
            )?;
            apply_exclusions(&mut result, &opts.exclude);
            output::print(&out_opts, &result)?;
            exit_if_assessment_fails(&result);
        }
        Commands::Release { arg, opts } => {
            let out_opts = output::OutputOptions {
                format: opts.format,
                only_failures: opts.only_failures,
            };
            let cfg = GitHubConfig::load()?;
            let (owner, repo_name) = resolve_repo(&cfg, opts.repo.as_deref())?;
            check_repo_exists(&owner, &repo_name)?;
            let client = GitHubClient::new(&cfg)?;

            let release_arg = match arg {
                Some(a) => a,
                None => detect_latest_release_tag(&client, &owner, &repo_name)
                    .context("could not detect latest release. Pass a tag explicitly")?,
            };
            let (base_tag, head_tag) =
                parse_release_arg(&release_arg, &client, &owner, &repo_name)?;

            eprintln!("Checking release: {base_tag}..{head_tag}");

            let mut result = verify_release(
                &client,
                &owner,
                &repo_name,
                &base_tag,
                &head_tag,
                opts.policy.as_deref(),
                opts.with_evidence,
            )?;
            apply_exclusions(&mut result, &opts.exclude);
            output::print(&out_opts, &result)?;
            exit_if_assessment_fails(&result);
        }
        Commands::Controls => {
            print_controls();
        }
        Commands::Policies => {
            print_policies();
        }
    }

    Ok(())
}

fn print_controls() {
    use colored::Colorize;

    let sections: &[(&str, &[(&str, &str)])] = &[
        (
            "SLSA Source Track",
            &[
                (
                    "source-authenticity",
                    "All commits are signed or verified (L1)",
                ),
                (
                    "review-independence",
                    "PRs reviewed by someone other than the author (L1)",
                ),
                (
                    "branch-history-integrity",
                    "Linear commit history without force pushes (L2)",
                ),
                (
                    "branch-protection-enforcement",
                    "Branch protection rules are enabled (L3)",
                ),
                (
                    "two-party-review",
                    "At least 2 independent reviewers approved (L4)",
                ),
            ],
        ),
        (
            "SLSA Build Track",
            &[
                (
                    "build-provenance",
                    "Build produces SLSA provenance attestation (L1)",
                ),
                (
                    "required-status-checks",
                    "Required CI checks pass on HEAD commit (L1)",
                ),
                (
                    "hosted-build-platform",
                    "Builds run on hosted (non-self-hosted) runners (L2)",
                ),
                (
                    "provenance-authenticity",
                    "Build provenance signatures are valid (L2)",
                ),
                (
                    "build-isolation",
                    "Builds run in ephemeral, isolated environments (L3)",
                ),
            ],
        ),
        (
            "SLSA Dependencies Track",
            &[
                (
                    "dependency-signature",
                    "Dependencies have valid signatures (L1)",
                ),
                (
                    "dependency-provenance",
                    "Dependencies publish provenance attestations (L2)",
                ),
                (
                    "dependency-signer-verified",
                    "Dependency signers match a trusted list (L3)",
                ),
                (
                    "dependency-completeness",
                    "All transitive dependencies have provenance (L4)",
                ),
            ],
        ),
        (
            "SOC2 CC7 (Traceability & Anomaly Detection)",
            &[
                (
                    "issue-linkage",
                    "PR references an issue (Fixes #N, Closes #N)",
                ),
                (
                    "release-traceability",
                    "Release linked to merged PRs and issues",
                ),
                ("stale-review", "No code pushed after last approval"),
                (
                    "security-file-change",
                    "Security-sensitive changes get extra review",
                ),
            ],
        ),
        (
            "SOC2 CC8 (Change Management)",
            &[
                (
                    "change-request-size",
                    "PR is reasonably sized (not too large)",
                ),
                (
                    "test-coverage",
                    "Changed source files have matching test updates",
                ),
                ("scoped-change", "PR contains a single logical change"),
                ("description-quality", "PR has a meaningful description"),
                (
                    "merge-commit-policy",
                    "Uses squash or rebase (no merge commits)",
                ),
                (
                    "conventional-title",
                    "Title follows Conventional Commits format",
                ),
            ],
        ),
        (
            "Repository Security",
            &[
                (
                    "codeowners-coverage",
                    "CODEOWNERS file defines code ownership",
                ),
                ("secret-scanning", "Secret scanning is enabled"),
                (
                    "vulnerability-scanning",
                    "Dependabot vulnerability alerts are enabled",
                ),
                (
                    "security-policy",
                    "SECURITY.md with disclosure process exists",
                ),
            ],
        ),
    ];

    for (title, controls) in sections {
        println!("{}", title.bold());
        for (id, desc) in *controls {
            println!("  {:<35} {desc}", id);
        }
        println!();
    }
}

fn print_policies() {
    use colored::Colorize;
    println!("{}", "Available policy presets:".bold());
    println!();

    let presets: &[(&str, &str)] = &[
        (
            "default",
            "All controls strict \u{2014} indeterminate/violated maps to fail",
        ),
        (
            "oss",
            "Tolerates unsigned commits and self-reviewed merges (review instead of fail)",
        ),
        (
            "aiops",
            "Escalates all indeterminate to human review instead of fail",
        ),
        (
            "soc1",
            "Strict on ICFR-relevant controls; advisory on compliance controls",
        ),
        (
            "soc2",
            "Strict on CC6/CC7/CC8 controls; review on build-track indeterminate",
        ),
        (
            "slsa-l1",
            "Enforce SLSA source/build/dependencies at Level 1",
        ),
        (
            "slsa-l2",
            "Enforce SLSA source/build/dependencies at Level 2",
        ),
        (
            "slsa-l3",
            "Enforce SLSA source/build/dependencies at Level 3",
        ),
        (
            "slsa-l4",
            "Enforce SLSA source/build/dependencies at Level 4",
        ),
    ];

    for (name, desc) in presets {
        println!("  {:<12} {desc}", name.bold());
    }

    println!();
    println!("Usage: gh verify pr 42 --policy <PRESET>");
    println!("       gh verify pr 42 --policy ./custom.rego");
    println!();
    println!("See docs/custom-policies.md for custom policy authoring.");
}

fn apply_exclusions(
    result: &mut libverify_core::assessment::VerificationResult,
    exclude: &[String],
) {
    if exclude.is_empty() {
        return;
    }
    result
        .report
        .outcomes
        .retain(|o| !exclude.iter().any(|e| o.control_id.to_string() == *e));
    result
        .report
        .findings
        .retain(|f| !exclude.iter().any(|e| f.control_id.to_string() == *e));
}

fn apply_batch_exclusions(batch: &mut libverify_core::assessment::BatchReport, exclude: &[String]) {
    if exclude.is_empty() {
        return;
    }
    for entry in &mut batch.reports {
        apply_exclusions(&mut entry.result, exclude);
    }
}

fn check_repo_exists(owner: &str, repo: &str) -> Result<()> {
    let output = std::process::Command::new("gh")
        .args(["api", &format!("repos/{owner}/{repo}"), "--silent"])
        .stderr(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .output()
        .context("failed to run 'gh api' — is the GitHub CLI installed?")?;
    if !output.status.success() {
        anyhow::bail!(
            "repository '{owner}/{repo}' not found or not accessible. Check the name and your permissions"
        );
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
        if let Some(path) = path
            && path.matches('/').count() == 1
            && !path.starts_with('/')
        {
            return Some(path.to_string());
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
        anyhow::bail!(
            "no PR number provided and no open PR found for current branch. Specify a PR number: gh verify pr 123"
        );
    }
    let stdout = String::from_utf8(output.stdout).context("gh pr view returned invalid UTF-8")?;
    stdout
        .trim()
        .parse::<u32>()
        .with_context(|| format!("gh pr view returned unexpected output: {:?}", stdout.trim()))
}

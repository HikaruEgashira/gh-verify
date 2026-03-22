use std::process;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};

use gh_verify::adapters;
use gh_verify::attestation;
use gh_verify::config::Config;
use gh_verify::github;
use gh_verify::github::client::GitHubClient;
use gh_verify::output;
use gh_verify::policy::OpaProfile;
use gh_verify_core::evidence::{EvidenceGap, EvidenceState};
use gh_verify_core::slsa::SlsaLevel;

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
        /// PR number (omit to detect from current branch)
        arg: Option<String>,
        /// Output format (human, json, or sarif)
        #[arg(long, default_value = "human")]
        format: String,
        /// Repository in OWNER/REPO format
        #[arg(long)]
        repo: Option<String>,
        /// OPA policy: preset name (default, oss, aiops, soc1, soc2) or .rego file path
        #[arg(long)]
        policy: Option<String>,
        /// SLSA level target: source-l{N}-build-l{M} (e.g. "source-l3-build-l2")
        #[arg(long)]
        slsa_level: Option<String>,
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
        /// OPA policy: preset name (default, oss, aiops, soc1, soc2) or .rego file path
        #[arg(long)]
        policy: Option<String>,
        /// SLSA level target: source-l{N}-build-l{M} (e.g. "source-l3-build-l2")
        #[arg(long)]
        slsa_level: Option<String>,
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
            slsa_level,
        } => {
            let pr_number: u32 = match arg {
                Some(a) => a.parse().context("invalid PR number")?,
                None => detect_pr_number().context(
                    "could not detect PR for current branch. Pass a PR number explicitly",
                )?,
            };
            let fmt = output::parse_format(&format)?;
            let cfg = Config::load()?;
            let (owner, repo_name) = resolve_repo(&cfg, repo_override.as_deref())?;
            let client = GitHubClient::new(&cfg)?;

            let pr_files = github::pr_api::get_pr_files(&client, &owner, &repo_name, pr_number)
                .context("failed to fetch PR files")?;
            let pr_metadata =
                github::pr_api::get_pr_metadata(&client, &owner, &repo_name, pr_number)
                    .context("failed to fetch PR metadata")?;
            let pr_reviews = github::pr_api::get_pr_reviews(&client, &owner, &repo_name, pr_number)
                .context("failed to fetch PR reviews")?;
            let pr_commits = github::pr_api::get_pr_commits(&client, &owner, &repo_name, pr_number)
                .context("failed to fetch PR commits")?;

            let head_sha = &pr_metadata.head.sha;
            let check_runs_evidence =
                fetch_check_runs_evidence(&client, &owner, &repo_name, head_sha);

            let repo_full = format!("{owner}/{repo_name}");
            let mut bundle = adapters::github::build_pull_request_bundle(
                &repo_full,
                pr_number,
                &pr_metadata,
                &pr_files,
                &pr_reviews,
                &pr_commits,
            );
            bundle.check_runs = check_runs_evidence;

            // Build platform evidence from check runs
            if let Some(cr_list) = bundle.check_runs.value() {
                let build_platforms = adapters::github::map_build_platform_evidence(cr_list);
                if !build_platforms.is_empty() {
                    bundle.build_platform = EvidenceState::complete(build_platforms);
                }
            }

            let report = assess_bundle(&bundle, policy.as_deref(), slsa_level.as_deref())?;
            output::print(fmt, &report)?;
            exit_if_assessment_fails(&report);
        }
        Commands::Release {
            arg,
            format,
            repo: repo_override,
            policy,
            slsa_level,
        } => {
            let fmt = output::parse_format(&format)?;
            let cfg = Config::load()?;
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

            let commits = github::release_api::compare_refs(
                &client, &owner, &repo_name, &base_tag, &head_tag,
            )
            .context("failed to compare refs")?;

            if commits.is_empty() {
                println!("No commits found between {base_tag} and {head_tag}");
                return Ok(());
            }
            println!("Found {} commits", commits.len());

            let mut commit_prs = Vec::new();

            for c in &commits {
                let prs =
                    github::release_api::get_commit_pulls(&client, &owner, &repo_name, &c.sha)
                        .unwrap_or_else(|err| {
                            let short = gh_verify_core::integrity::short_sha(&c.sha);
                            eprintln!("Warning: failed to fetch PRs for commit {short}: {err}");
                            vec![]
                        });

                commit_prs.push(adapters::github::GitHubCommitPullAssociation {
                    commit_sha: c.sha.clone(),
                    pull_requests: prs,
                });
            }

            // Collect build-provenance attestations for release assets
            let release_assets =
                github::release_api::get_release_assets(&client, &owner, &repo_name, &head_tag)
                    .unwrap_or_else(|err| {
                        eprintln!("Warning: failed to fetch release assets: {err}");
                        vec![]
                    });

            let artifact_attestations = attestation::release::collect_release_attestations(
                &owner,
                &repo_name,
                &head_tag,
                &release_assets,
            );

            let repo_full = format!("{owner}/{repo_name}");
            let mut bundle = adapters::github::build_release_bundle(
                &repo_full,
                &base_tag,
                &head_tag,
                &commits,
                &commit_prs,
                artifact_attestations,
            );
            // Check runs are PR-scoped; not applicable for release verification.
            bundle.check_runs = EvidenceState::not_applicable();
            let report = assess_bundle(&bundle, policy.as_deref(), slsa_level.as_deref())?;
            output::print(fmt, &report)?;
            exit_if_assessment_fails(&report);
        }
    }

    Ok(())
}

fn resolve_repo(cfg: &Config, override_repo: Option<&str>) -> Result<(String, String)> {
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

fn detect_latest_release_tag(client: &GitHubClient, owner: &str, repo: &str) -> Result<String> {
    let tags = github::release_api::get_tags(client, owner, repo)?;
    tags.into_iter()
        .next()
        .map(|t| t.name)
        .context("no tags found in repository")
}

fn parse_release_arg(
    arg: &str,
    client: &GitHubClient,
    owner: &str,
    repo: &str,
) -> Result<(String, String)> {
    if let Some(sep_idx) = arg.find("..") {
        let base = arg[..sep_idx].to_string();
        let head = arg[sep_idx + 2..].to_string();
        return Ok((base, head));
    }

    let head_tag = arg.to_string();
    let tags = github::release_api::get_tags(client, owner, repo)?;

    for (idx, t) in tags.iter().enumerate() {
        if t.name == head_tag {
            if idx + 1 < tags.len() {
                return Ok((tags[idx + 1].name.clone(), head_tag));
            } else {
                bail!("no previous tag found before {head_tag}");
            }
        }
    }
    bail!("tag not found: {head_tag}");
}

fn fetch_check_runs_evidence(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    git_ref: &str,
) -> EvidenceState<Vec<gh_verify_core::evidence::CheckRunEvidence>> {
    let check_runs_result = github::pr_api::get_commit_check_runs(client, owner, repo, git_ref);
    let combined_status_result = github::pr_api::get_commit_status(client, owner, repo, git_ref);

    match check_runs_result {
        Ok(cr_response) => {
            let combined = combined_status_result.ok();
            let evidence = adapters::github::map_check_runs_evidence(
                &cr_response.check_runs,
                combined.as_ref(),
            );
            EvidenceState::complete(evidence)
        }
        Err(e) => EvidenceState::missing(vec![EvidenceGap::CollectionFailed {
            source: "github".to_string(),
            subject: format!("commits/{git_ref}/check-runs"),
            detail: format!("{e:#}"),
        }]),
    }
}

/// Parse a SLSA level string like "source-l3-build-l2" into (source_level, build_level).
fn parse_slsa_level(s: &str) -> Result<(SlsaLevel, SlsaLevel)> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 4 || parts[0] != "source" || parts[2] != "build" {
        bail!(
            "invalid --slsa-level format: expected 'source-l{{N}}-build-l{{M}}' (e.g. 'source-l3-build-l2'), got '{s}'"
        );
    }

    let source_level = parse_level_component(parts[1])
        .with_context(|| format!("invalid source level in '{s}'"))?;
    let build_level =
        parse_level_component(parts[3]).with_context(|| format!("invalid build level in '{s}'"))?;

    Ok((source_level, build_level))
}

fn parse_level_component(s: &str) -> Result<SlsaLevel> {
    match s {
        "l0" => Ok(SlsaLevel::L0),
        "l1" => Ok(SlsaLevel::L1),
        "l2" => Ok(SlsaLevel::L2),
        "l3" => Ok(SlsaLevel::L3),
        "l4" => Ok(SlsaLevel::L4),
        _ => bail!("unknown level '{s}': expected l0, l1, l2, l3, or l4"),
    }
}

fn assess_bundle(
    bundle: &gh_verify_core::evidence::EvidenceBundle,
    policy_path: Option<&str>,
    slsa_level: Option<&str>,
) -> Result<gh_verify_core::assessment::AssessmentReport> {
    // --policy takes precedence over --slsa-level
    match policy_path {
        Some(name) => {
            let profile = OpaProfile::from_preset_or_file(name)?;
            let controls = gh_verify_core::controls::all_controls();
            Ok(gh_verify_core::assessment::assess(
                bundle, &controls, &profile,
            ))
        }
        None => match slsa_level {
            Some(level_str) => {
                let (source_level, build_level) = parse_slsa_level(level_str)?;
                Ok(gh_verify_core::assessment::assess_with_slsa_levels(
                    bundle,
                    source_level,
                    build_level,
                ))
            }
            None => Ok(gh_verify_core::assessment::assess_all_controls_with_levels(
                bundle,
                gh_verify_core::slsa::SlsaLevel::L1,
                gh_verify_core::slsa::SlsaLevel::L1,
            )),
        },
    }
}

fn exit_if_assessment_fails(report: &gh_verify_core::assessment::AssessmentReport) {
    if report
        .outcomes
        .iter()
        .any(|o| o.decision == gh_verify_core::profile::GateDecision::Fail)
    {
        process::exit(1);
    }
}

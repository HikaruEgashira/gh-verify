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

const VERSION: &str = env!("GH_VERIFY_VERSION");

#[derive(Parser)]
#[command(name = "gh-verify", version = VERSION, about = "SLSA-based GitHub SDLC health checker")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Verify a pull request
    Pr {
        /// PR number
        arg: String,
        /// Output format (human, json, or sarif)
        #[arg(long, default_value = "human")]
        format: String,
        /// Repository in OWNER/REPO format
        #[arg(long)]
        repo: Option<String>,
        /// Path to OPA policy file (.rego) for custom gate decisions
        #[arg(long)]
        policy: Option<String>,
    },
    /// Verify release integrity
    Release {
        /// Tag or BASE..HEAD range
        arg: String,
        /// Output format (human, json, or sarif)
        #[arg(long, default_value = "human")]
        format: String,
        /// Repository in OWNER/REPO format
        #[arg(long)]
        repo: Option<String>,
        /// Path to OPA policy file (.rego) for custom gate decisions
        #[arg(long)]
        policy: Option<String>,
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
        } => {
            let pr_number: u32 = arg.parse().context("invalid PR number")?;
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

            let status_checks = fetch_required_status_checks(&client, &owner, &repo_name);

            let repo_full = format!("{owner}/{repo_name}");
            let mut bundle = adapters::github::build_pull_request_bundle(
                &repo_full,
                pr_number,
                &pr_metadata,
                &pr_files,
                &pr_reviews,
                &pr_commits,
            );
            bundle.required_status_checks = status_checks;
            let report = assess_bundle(&bundle, policy.as_deref())?;
            output::print(fmt, &report)?;
            exit_if_assessment_fails(&report);
        }
        Commands::Release {
            arg,
            format,
            repo: repo_override,
            policy,
        } => {
            let fmt = output::parse_format(&format)?;
            let cfg = Config::load()?;
            let (owner, repo_name) = resolve_repo(&cfg, repo_override.as_deref())?;
            let client = GitHubClient::new(&cfg)?;

            let (base_tag, head_tag) = parse_release_arg(&arg, &client, &owner, &repo_name)?;

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

            let status_checks = fetch_required_status_checks(&client, &owner, &repo_name);

            // Collect build-provenance attestations for release assets
            let release_assets = github::release_api::get_release_assets(
                &client, &owner, &repo_name, &head_tag,
            )
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
            bundle.required_status_checks = status_checks;
            let report = assess_bundle(&bundle, policy.as_deref())?;
            output::print(fmt, &report)?;
            exit_if_assessment_fails(&report);
        }
    }

    Ok(())
}

fn resolve_repo<'a>(cfg: &'a Config, override_repo: Option<&'a str>) -> Result<(String, String)> {
    let repo_str = override_repo.unwrap_or(&cfg.repo);
    let slash_idx = repo_str
        .find('/')
        .context("could not resolve repo. Use --repo OWNER/REPO or set GH_REPO env var")?;
    let owner = repo_str[..slash_idx].to_string();
    let repo_name = repo_str[slash_idx + 1..].to_string();
    Ok((owner, repo_name))
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

fn fetch_required_status_checks(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
) -> EvidenceState<Vec<String>> {
    match github::repo_api::get_default_branch(client, owner, repo)
        .and_then(|branch| github::repo_api::get_branch_protection(client, owner, repo, &branch))
    {
        Ok(bp_response) => adapters::github::map_required_status_checks_evidence(&bp_response),
        Err(e) => EvidenceState::missing(vec![EvidenceGap::CollectionFailed {
            source: "github".to_string(),
            subject: "branch-protection".to_string(),
            detail: format!("{e:#}"),
        }]),
    }
}

fn assess_bundle(
    bundle: &gh_verify_core::evidence::EvidenceBundle,
    policy_path: Option<&str>,
) -> Result<gh_verify_core::assessment::AssessmentReport> {
    match policy_path {
        Some(name) => {
            let profile = OpaProfile::from_preset_or_file(name)?;
            let controls = gh_verify_core::controls::slsa_foundation_controls();
            Ok(gh_verify_core::assessment::assess(bundle, &controls, &profile))
        }
        None => Ok(gh_verify_core::assessment::assess_with_slsa_foundation(bundle)),
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

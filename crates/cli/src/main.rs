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
        /// SLSA level target: source-l{N}-build-l{M} (e.g. "source-l3-build-l2")
        #[arg(long)]
        slsa_level: Option<String>,
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

            // Fetch branch protection for the base branch
            let base_branch = &pr_metadata.base.ref_name;
            bundle.branch_protection =
                fetch_branch_protection_evidence(&client, &owner, &repo_name, base_branch);

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

fn fetch_branch_protection_evidence(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    branch: &str,
) -> EvidenceState<Vec<gh_verify_core::evidence::BranchProtectionEvidence>> {
    match github::repo_api::get_branch_protection(client, owner, repo, branch) {
        Ok(response) => {
            let evidence = adapters::github::map_branch_protection_evidence(&response, branch);
            EvidenceState::complete(vec![evidence])
        }
        Err(e) => {
            eprintln!("Warning: failed to fetch branch protection for {branch}: {e:#}");
            EvidenceState::missing(vec![EvidenceGap::CollectionFailed {
                source: "github".to_string(),
                subject: format!("branches/{branch}/protection"),
                detail: format!("{e:#}"),
            }])
        }
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

use std::process;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};

use gh_verify::adapters;
use gh_verify::config::Config;
use gh_verify::github;
use gh_verify::github::client::GitHubClient;
use gh_verify::output;
use gh_verify::policy::OpaProfile;

use gh_verify_core::evidence::{EvidenceBundle, EvidenceState};

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
        /// Output format (human or json)
        #[arg(long, default_value = "human")]
        format: String,
        /// Repository in OWNER/REPO format
        #[arg(long)]
        repo: Option<String>,
        /// Policy: built-in name (slsa-foundation, slsa-comprehensive) or path to .rego file
        #[arg(long, default_value = "slsa-foundation")]
        policy: String,
    },
    /// Verify release integrity
    Release {
        /// Tag or BASE..HEAD range
        arg: String,
        /// Output format (human or json)
        #[arg(long, default_value = "human")]
        format: String,
        /// Repository in OWNER/REPO format
        #[arg(long)]
        repo: Option<String>,
        /// Policy: built-in name (slsa-foundation, slsa-comprehensive) or path to .rego file
        #[arg(long, default_value = "slsa-foundation")]
        policy: String,
    },
    /// Verify artifact attestation (binary, OCI image, or SBOM)
    Attest {
        /// Path to artifact, or oci://REGISTRY/IMAGE:TAG for container images
        artifact: String,
        /// Owner for attestation lookup
        #[arg(long)]
        owner: Option<String>,
        /// Repository in OWNER/REPO format (more precise than --owner)
        #[arg(long)]
        repo: Option<String>,
        /// Digest algorithm for artifact hashing
        #[arg(long, default_value = "sha256", value_parser = ["sha256", "sha512"])]
        digest_alg: String,
        /// Attestation predicate type to verify
        #[arg(long, default_value = "https://slsa.dev/provenance/v1")]
        predicate_type: String,
        /// Require attestation signed by this specific workflow
        #[arg(long)]
        signer_workflow: Option<String>,
        /// Reject attestations from self-hosted runners
        #[arg(long)]
        deny_self_hosted_runners: bool,
        /// Output format (human or json)
        #[arg(long, default_value = "human")]
        format: String,
        /// Policy: built-in name (slsa-foundation, slsa-comprehensive) or path to .rego file
        #[arg(long, default_value = "slsa-comprehensive")]
        policy: String,
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

            let repo_full = format!("{owner}/{repo_name}");
            let mut bundle = adapters::github::build_pull_request_bundle(
                &repo_full,
                pr_number,
                &pr_metadata,
                &pr_files,
                &pr_reviews,
                &pr_commits,
            );

            if is_comprehensive(&policy) {
                collect_repo_policy(&client, &owner, &repo_name, &mut bundle);
            }

            process_assessment(&bundle, &policy, &format)
        }
        Commands::Release {
            arg,
            format,
            repo: repo_override,
            policy,
        } => {
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

            let repo_full = format!("{owner}/{repo_name}");
            let mut bundle = adapters::github::build_release_bundle(
                &repo_full,
                &base_tag,
                &head_tag,
                &commits,
                &commit_prs,
            );

            if is_comprehensive(&policy) {
                collect_repo_policy(&client, &owner, &repo_name, &mut bundle);
            }

            process_assessment(&bundle, &policy, &format)
        }
        Commands::Attest {
            artifact,
            owner,
            repo: repo_override,
            digest_alg,
            predicate_type,
            signer_workflow,
            deny_self_hosted_runners,
            format,
            policy,
        } => {
            let (owner_name, repo_name) = if let Some(ref r) = repo_override {
                let parts: Vec<&str> = r.splitn(2, '/').collect();
                if parts.len() != 2 {
                    bail!("--repo must be in OWNER/REPO format");
                }
                (parts[0].to_string(), Some(parts[1].to_string()))
            } else if let Some(ref o) = owner {
                (o.clone(), None)
            } else {
                let cfg = Config::load()?;
                let (o, r) = resolve_repo(&cfg, None)?;
                (o, Some(r))
            };

            let repo_full = repo_name.as_ref().map(|rn| format!("{owner_name}/{rn}"));

            let attestations = gh_verify::attestation::gh_cli::verify_artifact_extended(
                &artifact,
                Some(&owner_name),
                repo_full.as_deref(),
                &digest_alg,
                &predicate_type,
                signer_workflow.as_deref(),
                deny_self_hosted_runners,
            )?;

            let artifact_evidence =
                gh_verify::attestation::gh_cli::to_artifact_attestations(&artifact, &attestations);

            let mut bundle = EvidenceBundle {
                artifact_attestations: EvidenceState::complete(artifact_evidence),
                ..Default::default()
            };

            if is_comprehensive(&policy) {
                if let Some(ref rn) = repo_name {
                    let cfg = Config::load()?;
                    let client = GitHubClient::new(&cfg)?;
                    collect_repo_policy(&client, &owner_name, rn, &mut bundle);
                }
            }

            process_assessment(&bundle, &policy, &format)
        }
    }
}

// --- Shared pipeline ---

/// Single point for policy resolution → assess → output → exit-code.
fn process_assessment(bundle: &EvidenceBundle, policy: &str, format: &str) -> Result<()> {
    let fmt = output::parse_format(format)?;
    let profile = OpaProfile::resolve(policy)?;
    let controls = if is_comprehensive(policy) {
        gh_verify_core::controls::slsa_comprehensive_controls()
    } else {
        gh_verify_core::controls::slsa_foundation_controls()
    };
    let report = gh_verify_core::assessment::assess(bundle, &controls, &profile);
    output::print(fmt, &report)?;
    if report
        .outcomes
        .iter()
        .any(|o| o.decision == gh_verify_core::profile::GateDecision::Fail)
    {
        process::exit(1);
    }
    Ok(())
}

/// Returns true if the policy implies comprehensive control set.
fn is_comprehensive(policy: &str) -> bool {
    policy != "slsa-foundation"
}

// --- Infrastructure ---

fn collect_repo_policy(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    bundle: &mut EvidenceBundle,
) {
    let default_branch = match github::repo_api::get_default_branch(client, owner, repo) {
        Ok(branch) => branch,
        Err(err) => {
            eprintln!("Warning: could not determine default branch: {err}");
            bundle.repository_policy = EvidenceState::missing(vec![
                gh_verify_core::evidence::EvidenceGap::CollectionFailed {
                    source: "github_api".to_string(),
                    subject: format!("{owner}/{repo}"),
                    detail: format!("default branch resolution failed: {err}"),
                },
            ]);
            return;
        }
    };

    match github::repo_api::get_branch_protection(client, owner, repo, &default_branch) {
        Ok(protection) => {
            let config = adapters::github::map_branch_protection_evidence(&protection);
            bundle.repository_policy =
                EvidenceState::complete(gh_verify_core::evidence::RepositoryPolicy {
                    branch_protection: EvidenceState::complete(config),
                    required_status_checks: EvidenceState::not_applicable(),
                });
        }
        Err(err) => {
            eprintln!("Warning: could not fetch branch protection: {err}");
            bundle.repository_policy = EvidenceState::missing(vec![
                gh_verify_core::evidence::EvidenceGap::CollectionFailed {
                    source: "github_api".to_string(),
                    subject: format!("{owner}/{repo}"),
                    detail: err.to_string(),
                },
            ]);
        }
    }
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

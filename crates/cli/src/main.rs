use std::collections::HashSet;
use std::process;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};

use gh_verify::adapters;
use gh_verify::config::Config;
use gh_verify::github;
use gh_verify::github::client::GitHubClient;
use gh_verify::output;
use gh_verify::rules;
use gh_verify::rules::engine;

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
        /// PR number or "list-rules"
        arg: String,
        /// Output format (human or json)
        #[arg(long, default_value = "human")]
        format: String,
        /// Repository in OWNER/REPO format
        #[arg(long)]
        repo: Option<String>,
        /// Disable detect-missing-test rule
        #[arg(long)]
        no_detect_missing_test: bool,
        /// Additional test filename pattern (comma-separated, `*` placeholder)
        #[arg(long, value_delimiter = ',')]
        test_pattern: Vec<String>,
        /// Path to LCOV coverage report file
        #[arg(long)]
        coverage: Option<String>,
        /// Use legacy rule engine instead of control/evidence assessment
        #[arg(long)]
        legacy_rules: bool,
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
        /// Use legacy rule engine instead of control/evidence assessment
        #[arg(long)]
        legacy_rules: bool,
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
            no_detect_missing_test,
            test_pattern,
            coverage,
            legacy_rules,
        } => {
            if arg == "list-rules" {
                println!("Available rules:");
                for id in engine::list_rule_ids() {
                    println!("  {id}");
                }
                return Ok(());
            }

            let pr_number: u32 = arg.parse().context("invalid PR number")?;
            let fmt = output::parse_format(&format)?;
            let cfg = Config::load()?;
            let (owner, repo_name) = resolve_repo(&cfg, repo_override.as_deref())?;
            let client = GitHubClient::new(&cfg)?;

            let coverage_report = coverage
                .map(|path| std::fs::read_to_string(&path))
                .transpose()
                .context("failed to read coverage report")?;

            let pr_files = github::pr_api::get_pr_files(&client, &owner, &repo_name, pr_number)
                .context("failed to fetch PR files")?;
            let pr_metadata =
                github::pr_api::get_pr_metadata(&client, &owner, &repo_name, pr_number)
                    .context("failed to fetch PR metadata")?;
            let pr_reviews = github::pr_api::get_pr_reviews(&client, &owner, &repo_name, pr_number)
                .context("failed to fetch PR reviews")?;
            let pr_commits = github::pr_api::get_pr_commits(&client, &owner, &repo_name, pr_number)
                .context("failed to fetch PR commits")?;

            if legacy_rules {
                let ctx = rules::RuleContext::Pr {
                    pr_files,
                    pr_metadata,
                    pr_reviews,
                    pr_commits,
                    options: rules::PrRuleOptions {
                        detect_missing_test: !no_detect_missing_test,
                        test_patterns: test_pattern,
                        coverage_report,
                    },
                };
                let results = engine::run_all(&ctx)?;
                output::print(fmt, &results)?;

                if results.iter().any(|r| r.severity.is_failing()) {
                    process::exit(1);
                }
            } else {
                let repo_full = format!("{owner}/{repo_name}");
                let bundle = adapters::github::build_pull_request_bundle(
                    &repo_full,
                    pr_number,
                    &pr_metadata,
                    &pr_files,
                    &pr_reviews,
                    &pr_commits,
                );
                let report = gh_verify_core::assessment::assess_with_slsa_foundation(&bundle);
                output::print_assessment(fmt, &report)?;
                exit_if_assessment_fails(&report);
            }
        }
        Commands::Release {
            arg,
            format,
            repo: repo_override,
            legacy_rules,
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

            // Fetch PR associations for each commit
            let mut commit_prs = Vec::new();
            let mut seen_prs = HashSet::new();

            for c in &commits {
                let prs =
                    github::release_api::get_commit_pulls(&client, &owner, &repo_name, &c.sha)
                        .unwrap_or_else(|err| {
                            let short = if c.sha.len() >= 7 {
                                &c.sha[..7]
                            } else {
                                &c.sha
                            };
                            eprintln!("Warning: failed to fetch PRs for commit {short}: {err}");
                            vec![]
                        });

                for pr in &prs {
                    seen_prs.insert(pr.number);
                }

                commit_prs.push(adapters::github::GitHubCommitPullAssociation {
                    commit_sha: c.sha.clone(),
                    pull_requests: prs,
                });
            }

            if legacy_rules {
                // Fetch reviews for each unique PR
                let mut pr_reviews = Vec::new();
                for pr_number in &seen_prs {
                    let pr_author = find_pr_author(&commit_prs, *pr_number);

                    let reviews = github::release_api::get_pr_reviews(
                        &client, &owner, &repo_name, *pr_number,
                    )
                    .unwrap_or_else(|err| {
                        eprintln!("Warning: failed to fetch reviews for PR #{pr_number}: {err}");
                        vec![]
                    });

                    pr_reviews.push(rules::PrReviewSet {
                        pr_number: *pr_number,
                        pr_author,
                        reviews,
                    });
                }

                let ctx = rules::RuleContext::Release {
                    base_tag,
                    head_tag,
                    commits,
                    commit_prs,
                    pr_reviews,
                };
                let results = engine::run_all(&ctx)?;
                output::print(fmt, &results)?;

                if results.iter().any(|r| r.severity.is_failing()) {
                    process::exit(1);
                }
            } else {
                let repo_full = format!("{owner}/{repo_name}");
                let bundle = adapters::github::build_release_bundle(
                    &repo_full,
                    &base_tag,
                    &head_tag,
                    &commits,
                    &commit_prs,
                );
                let report = gh_verify_core::assessment::assess_with_slsa_foundation(&bundle);
                output::print_assessment(fmt, &report)?;
                exit_if_assessment_fails(&report);
            }
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

fn exit_if_assessment_fails(report: &gh_verify_core::assessment::AssessmentReport) {
    if report
        .outcomes
        .iter()
        .any(|o| o.decision == gh_verify_core::profile::GateDecision::Fail)
    {
        process::exit(1);
    }
}

fn find_pr_author(
    commit_prs: &[adapters::github::GitHubCommitPullAssociation],
    pr_number: u32,
) -> String {
    for assoc in commit_prs {
        for pr in &assoc.pull_requests {
            if pr.number == pr_number {
                return pr.user.login.clone();
            }
        }
    }
    "unknown".to_string()
}

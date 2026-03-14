use std::collections::HashSet;
use std::process;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};

use gh_verify::config::Config;
use gh_verify::github;
use gh_verify::github::client::GitHubClient;
use gh_verify::output;
use gh_verify::rules;
use gh_verify::rules::engine;

const VERSION: &str = "0.2.0";

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

            let pr_files = github::pr_api::get_pr_files(&client, &owner, &repo_name, pr_number)
                .context("failed to fetch PR files")?;
            let pr_metadata =
                github::pr_api::get_pr_metadata(&client, &owner, &repo_name, pr_number)
                    .context("failed to fetch PR metadata")?;
            let pr_reviews =
                github::pr_api::get_pr_reviews(&client, &owner, &repo_name, pr_number)
                    .context("failed to fetch PR reviews")?;
            let pr_commits =
                github::pr_api::get_pr_commits(&client, &owner, &repo_name, pr_number)
                    .context("failed to fetch PR commits")?;

            let ctx = rules::RuleContext::Pr {
                pr_files,
                pr_metadata,
                pr_reviews,
                pr_commits,
            };
            let results = engine::run_all(&ctx)?;
            output::print(fmt, &results)?;

            if results.iter().any(|r| r.severity.is_failing()) {
                process::exit(1);
            }
        }
        Commands::Release {
            arg,
            format,
            repo: repo_override,
        } => {
            let fmt = output::parse_format(&format)?;
            let cfg = Config::load()?;
            let (owner, repo_name) = resolve_repo(&cfg, repo_override.as_deref())?;
            let client = GitHubClient::new(&cfg)?;

            let (base_tag, head_tag) = parse_release_arg(&arg, &client, &owner, &repo_name)?;

            println!("Checking release: {base_tag}..{head_tag}");

            let commits =
                github::release_api::compare_refs(&client, &owner, &repo_name, &base_tag, &head_tag)
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
                let prs = github::release_api::get_commit_pulls(
                    &client,
                    &owner,
                    &repo_name,
                    &c.sha,
                )
                .unwrap_or_else(|err| {
                    let short = if c.sha.len() >= 7 { &c.sha[..7] } else { &c.sha };
                    eprintln!("Warning: failed to fetch PRs for commit {short}: {err}");
                    vec![]
                });

                for pr in &prs {
                    seen_prs.insert(pr.number);
                }

                commit_prs.push(rules::CommitPrAssociation {
                    commit_sha: c.sha.clone(),
                    prs,
                });
            }

            // Fetch reviews for each unique PR
            let mut pr_reviews = Vec::new();
            for pr_number in &seen_prs {
                // Find PR author
                let pr_author = find_pr_author(&commit_prs, *pr_number);

                let reviews = github::release_api::get_pr_reviews(
                    &client,
                    &owner,
                    &repo_name,
                    *pr_number,
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

fn find_pr_author(commit_prs: &[rules::CommitPrAssociation], pr_number: u32) -> String {
    for assoc in commit_prs {
        for pr in &assoc.prs {
            if pr.number == pr_number {
                return pr.user.login.clone();
            }
        }
    }
    "unknown".to_string()
}

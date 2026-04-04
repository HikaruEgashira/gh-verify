use std::process;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use gh_verify::gh_cli::run_gh;

use libverify_github::range::{
    detect_latest_release_tag, parse_range, parse_release_arg, resolve_pr_numbers,
};
use libverify_github::verify::exit_if_assessment_fails;
use libverify_github::{
    GitHubClient, GitHubConfig, assess_bundle, assess_repo_bundle,
    collect_release_attestation_evidence, collect_release_pr_evidence,
    collect_release_repo_evidence, collect_repo_evidence, verify_pr, verify_pr_batch, verify_repo,
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
    /// Policy: preset name (default, oss, aiops, soc1, soc2, slsa-l1..l4) or .rego file path.
    /// Comma-separated for multi-policy matrix output (e.g. --policy soc2,slsa-l2).
    #[arg(long, value_delimiter = ',')]
    policy: Vec<String>,
    /// Include raw collected evidence in output (only affects json/sarif formats)
    #[arg(long)]
    with_evidence: bool,
    /// Only show failing controls in output
    #[arg(long)]
    only_failures: bool,
    /// Exclude specific controls from results (comma-separated; see 'gh verify controls' for valid IDs)
    #[arg(long, value_delimiter = ',')]
    exclude: Vec<String>,
    /// Include only specific controls in results (comma-separated; see 'gh verify controls' for valid IDs)
    #[arg(long, value_delimiter = ',', conflicts_with = "exclude")]
    only: Vec<String>,
    /// Suppress progress messages on stderr (useful for CI/CD pipelines)
    #[arg(long, short)]
    quiet: bool,
    /// Report results without failing on violations (always exit 0). Useful for audits and incident response
    #[arg(long)]
    audit: bool,
    /// Write output to a file instead of stdout
    #[arg(long, short = 'o')]
    output_file: Option<String>,
}

impl CommonOpts {
    /// Return the single policy name for legacy (non-matrix) paths.
    /// Returns None (meaning "default") when no --policy is given.
    fn single_policy(&self) -> Option<&str> {
        match self.policy.as_slice() {
            [] => None,
            [one] => Some(one.as_str()),
            _ => Some(self.policy[0].as_str()),
        }
    }

    /// Return the list of policies, falling back to ["default"].
    fn policies(&self) -> Vec<&str> {
        if self.policy.is_empty() {
            vec!["default"]
        } else {
            self.policy.iter().map(|s| s.as_str()).collect()
        }
    }

    /// Whether this invocation requests multi-policy matrix output.
    fn is_matrix(&self) -> bool {
        self.policy.len() > 1
    }
}

#[derive(Parser)]
#[command(name = "gh-verify", version = VERSION,
    about = "GitHub SDLC health checker",
    long_about = "Verify pull requests, releases, and repositories against SDLC compliance controls.\nCatches common issues like missing reviews, unsigned commits, and oversized PRs.\nSupports SLSA source/build integrity, SOC2 CC7/CC8, dependency signatures, and more.",
    after_help = "Examples:\n  gh verify pr 42                    Verify PR #42 in current repo\n  gh verify pr 42 --audit            Report without failing (dry-run)\n  gh verify pr '#100..#200'           Batch verify a PR range\n  gh verify release                   Verify the latest release\n  gh verify repo --ref main           Check repository security\n  gh verify pr 42 --policy oss        Use OSS-friendly policy\n  gh verify pr 42 --format sarif      Output SARIF for code scanning\n  gh verify controls                  List all available controls\n  gh verify policies                  List all policy presets\n\nExit codes: 0 = pass, 1 = verification failure, 2 = infrastructure error\n\nOn PowerShell, use double quotes for ranges: gh verify pr \"#100..#200\""
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Verify a pull request
    #[command(
        after_help = "Examples:\n  gh verify pr 42                         Single PR\n  gh verify pr '#100..#200' --policy oss   All merged PRs in range\n  gh verify pr 42 --format json             JSON output\n\nRanges verify all merged PRs within the specified bounds."
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
        after_help = "Examples:\n  gh verify repo\n  gh verify repo --ref main --policy soc2\n  gh verify repo --format sarif\n  gh verify repo --repos org/api,org/web --policy soc2,slsa-l2"
    )]
    Repo {
        /// Git reference (branch, tag, or SHA). Defaults to HEAD.
        #[arg(long, default_value = "HEAD")]
        r#ref: String,
        /// Verify multiple repositories (comma-separated OWNER/REPO).
        /// Combined with multiple --policy values, produces a fleet matrix.
        #[arg(long, value_delimiter = ',')]
        repos: Vec<String>,
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
    Controls {
        /// Output format (human or json)
        #[arg(long, default_value_t = output::Format::Human)]
        format: output::Format,
    },
    /// List available policy presets and their descriptions
    Policies {
        /// Output format (human or json)
        #[arg(long, default_value_t = output::Format::Human)]
        format: output::Format,
    },
}

fn main() {
    // Disable ANSI colors when stdout is not a TTY (e.g., piped output)
    if !std::io::IsTerminal::is_terminal(&std::io::stdout()) {
        colored::control::set_override(false);
    }

    if let Err(e) = run() {
        eprintln!("Error: {e:#}");
        // Exit code 2 for infrastructure/configuration errors (distinct from exit 1 for verification failures)
        process::exit(2);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Pr { arg, opts } => {
            let out_opts = output::OutputOptions {
                format: opts.format,
                only_failures: opts.only_failures,
                policy: opts.policy.clone(),
                excluded: opts.exclude.clone(),
                output_file: opts.output_file.clone(),
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
                    if !opts.quiet {
                        eprintln!("Found {} PRs to verify", pr_numbers.len());
                    }
                    let mut batch = verify_pr_batch(
                        &client,
                        &owner,
                        &repo_name,
                        &pr_numbers,
                        opts.single_policy(),
                        opts.with_evidence,
                        Vec::new,
                    )?;
                    apply_batch_exclusions(&mut batch, &opts.exclude);
                    apply_batch_only_filter(&mut batch, &opts.only);
                    recalculate_batch_totals(&mut batch);
                    output::print_batch(&out_opts, &batch)?;
                    if !opts.audit && batch.total_fail > 0 {
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
                        opts.single_policy(),
                        opts.with_evidence,
                        vec![],
                    )?;
                    apply_exclusions(&mut result, &opts.exclude);
                    apply_only_filter(&mut result, &opts.only);
                    output::print(&out_opts, &result)?;
                    if !opts.audit {
                        exit_if_assessment_fails(&result);
                    }
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
                policy: vec![],
                excluded: vec![],
                output_file: None,
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
                // fmt is a pure format converter; exit code reflects the original command, not fmt
            }
        }
        Commands::Repo {
            r#ref: reference,
            repos,
            opts,
        } => {
            let out_opts = output::OutputOptions {
                format: opts.format,
                only_failures: opts.only_failures,
                policy: opts.policy.clone(),
                excluded: opts.exclude.clone(),
                output_file: opts.output_file.clone(),
            };

            // Fleet matrix mode: multiple repos and/or multiple policies
            if !repos.is_empty() || opts.is_matrix() {
                let cfg = GitHubConfig::load()?;
                let client = GitHubClient::new(&cfg)?;
                let policies = opts.policies();

                let repo_list: Vec<(String, String)> = if repos.is_empty() {
                    let (owner, repo_name) = resolve_repo(&cfg, opts.repo.as_deref())?;
                    vec![(owner, repo_name)]
                } else {
                    repos
                        .iter()
                        .map(|r| {
                            let (o, n) = resolve_repo(&cfg, Some(r.as_str()))?;
                            Ok((o, n))
                        })
                        .collect::<Result<Vec<_>>>()?
                };

                let matrix = run_fleet_matrix(
                    &client,
                    &repo_list,
                    &policies,
                    &reference,
                    &opts.exclude,
                    &opts.only,
                    opts.quiet,
                )?;

                output::print_fleet_matrix(&out_opts, &matrix)?;

                if !opts.audit && matrix.has_failures() {
                    process::exit(1);
                }
            } else {
                let cfg = GitHubConfig::load()?;
                let (owner, repo_name) = resolve_repo(&cfg, opts.repo.as_deref())?;
                check_repo_exists(&owner, &repo_name)?;
                let client = GitHubClient::new(&cfg)?;

                if !opts.quiet {
                    let policy_name = opts.single_policy().unwrap_or("default");
                    eprintln!("Checking security at ref: {reference} (policy: {policy_name})");
                }

                let mut result = verify_repo(
                    &client,
                    &owner,
                    &repo_name,
                    &reference,
                    opts.single_policy(),
                    opts.with_evidence,
                    vec![],
                )?;
                apply_exclusions(&mut result, &opts.exclude);
                apply_only_filter(&mut result, &opts.only);
                output::print(&out_opts, &result)?;
                if !opts.audit {
                    exit_if_assessment_fails(&result);
                }
            }
        }
        Commands::Release { arg, opts } => {
            let out_opts = output::OutputOptions {
                format: opts.format,
                only_failures: opts.only_failures,
                policy: opts.policy.clone(),
                excluded: opts.exclude.clone(),
                output_file: opts.output_file.clone(),
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

            let policy = opts.single_policy();
            let quiet = opts.quiet;

            if !quiet {
                eprintln!(
                    "Checking release: {base_tag}..{head_tag} (policy: {})",
                    policy.unwrap_or("default")
                );
            }

            let mut bundle =
                collect_release_pr_evidence(&client, &owner, &repo_name, &base_tag, &head_tag)?;
            collect_release_repo_evidence(&client, &owner, &repo_name, &head_tag, &mut bundle);
            collect_release_attestation_evidence(
                &client,
                &owner,
                &repo_name,
                &head_tag,
                &mut bundle,
            );

            // Final assessment and output
            let report = assess_bundle(&bundle, policy, vec![])?;
            let evidence_bundle = if opts.with_evidence {
                Some(bundle)
            } else {
                None
            };
            let mut result =
                libverify_core::assessment::VerificationResult::new(report, evidence_bundle);
            apply_exclusions(&mut result, &opts.exclude);
            apply_only_filter(&mut result, &opts.only);
            output::print(&out_opts, &result)?;
            if !opts.audit {
                exit_if_assessment_fails(&result);
            }
        }
        Commands::Controls { format } => match format {
            output::Format::Human => print_controls(),
            _ => print_controls_json(),
        },
        Commands::Policies { format } => match format {
            output::Format::Human => print_policies(),
            _ => print_policies_json(),
        },
    }

    Ok(())
}

fn print_controls() {
    println!(
        "{}",
        "SLSA levels: L1 (basic) → L2 (attested) → L3 (hardened) → L4 (comprehensive)".dimmed()
    );
    println!();

    for section in CONTROLS {
        println!("{}", section.title.bold());
        for (id, desc) in section.controls {
            println!("  {:<35} {desc}", id);
        }
        println!();
    }
}

fn print_policies() {
    use colored::Colorize;
    println!("{}", "Available policy presets:".bold());
    println!();

    for (name, desc) in POLICIES {
        println!("  {:<12} {desc}", name.bold());
    }

    println!();
    println!("Usage: gh verify pr 42 --policy <PRESET>");
    println!("       gh verify pr 42 --policy ./custom.rego");
    println!();
    println!("See docs/custom-policies.md for custom policy authoring.");
}

struct ControlSection {
    title: &'static str,
    controls: &'static [(&'static str, &'static str)],
}

const CONTROLS: &[ControlSection] = &[
    ControlSection {
        title: "SLSA Source Track",
        controls: &[
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
    },
    ControlSection {
        title: "SLSA Build Track",
        controls: &[
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
    },
    ControlSection {
        title: "SLSA Dependencies Track",
        controls: &[
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
    },
    ControlSection {
        title: "SOC2 CC7 (Traceability & Anomaly Detection)",
        controls: &[
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
    },
    ControlSection {
        title: "SOC2 CC8 (Change Management)",
        controls: &[
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
    },
    ControlSection {
        title: "Repository Security",
        controls: &[
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
            (
                "workflow-permissions-restricted",
                "Default workflow permissions are read-only",
            ),
            (
                "dependency-update-tool",
                "Dependabot or Renovate is configured",
            ),
            (
                "repository-permissions-audit",
                "Repository access follows least-privilege (limited admins, team-based)",
            ),
            (
                "default-branch-settings-baseline",
                "Default branch has protection baseline (protection + admin enforcement + stale dismissal)",
            ),
            (
                "security-test-in-ci",
                "Security testing (SAST/DAST) is active in CI pipelines",
            ),
            (
                "protected-tags",
                "Tag protection rules prevent unauthorized release tags",
            ),
        ],
    },
];

fn print_controls_json() {
    let items: Vec<serde_json::Value> = CONTROLS
        .iter()
        .flat_map(|section| {
            section.controls.iter().map(move |(id, desc)| {
                serde_json::json!({
                    "id": id,
                    "description": desc,
                    "section": section.title
                })
            })
        })
        .collect();
    println!("{}", serde_json::to_string_pretty(&items).unwrap());
}

const POLICIES: &[(&str, &str)] = &[
    (
        "default",
        "All controls strict \u{2014} uncertain or non-compliant results map to fail",
    ),
    (
        "oss",
        "OSS/solo-dev: relaxes unsigned commits, self-review, security-policy, stale-review to review",
    ),
    (
        "aiops",
        "AI-driven SDLC: uncertain\u{2192}review, dev-quality violated\u{2192}review for AI-generated PRs",
    ),
    (
        "soc1",
        "SOC1 (SSAE 18): strict on ICFR controls; non-ICFR controls are out-of-scope (review)",
    ),
    (
        "soc2",
        "SOC2 (TSC): strict on CC6/CC7/CC8/PI; commit signing and security-policy are advisory",
    ),
    (
        "slsa-l1",
        "SLSA v1.2 L1: requires build-provenance + dependency-signature only",
    ),
    (
        "slsa-l2",
        "SLSA v1.2 L2: adds history, hosted-build, vuln-scanning to L1",
    ),
    (
        "slsa-l3",
        "SLSA v1.2 L3: adds branch-protection, build-isolation, dep-provenance to L2",
    ),
    (
        "slsa-l4",
        "SLSA v1.2 L4: adds two-party-review, dep-completeness to L3 (maximum)",
    ),
    (
        "ismap",
        "ISMAP (ISO 27001): strict on operational/dev controls; review on advisory items",
    ),
    (
        "pci-dss",
        "PCI DSS v4.0 Req 6: strict on review/vuln/access; dev-quality is advisory",
    ),
    (
        "tisax",
        "TISAX (VDA ISA AL3): strict on supply-chain/source; SBOM/license are recommended",
    ),
    (
        "nist-800-53",
        "NIST 800-53 Rev.5 Moderate: strict on CM/SA/SI/SR; audit/dev-quality are advisory",
    ),
    (
        "wp29",
        "UN-R155 (WP.29): automotive CSMS; strict on supply-chain/build controls",
    ),
    (
        "scorecard",
        "OpenSSF Scorecard: critical/high\u{2192}fail, medium\u{2192}fail/review, unmapped\u{2192}review",
    ),
];

fn print_policies_json() {
    let items: Vec<serde_json::Value> = POLICIES
        .iter()
        .map(|(name, desc)| {
            serde_json::json!({
                "name": name,
                "description": desc
            })
        })
        .collect();
    println!("{}", serde_json::to_string_pretty(&items).unwrap());
}

fn apply_exclusions(
    result: &mut libverify_core::assessment::VerificationResult,
    exclude: &[String],
) {
    if exclude.is_empty() {
        return;
    }
    let exclude_set: std::collections::HashSet<&str> = exclude.iter().map(String::as_str).collect();
    let known_ids: std::collections::HashSet<String> = result
        .report
        .outcomes
        .iter()
        .map(|o| o.control_id.to_string())
        .collect();
    for e in exclude {
        if !known_ids.contains(e.as_str()) {
            eprintln!(
                "warning: unknown control ID '{}' in --exclude (see 'gh verify controls' for valid IDs)",
                e
            );
        }
    }
    result
        .report
        .outcomes
        .retain(|o| !exclude_set.contains(o.control_id.as_ref()));
    result
        .report
        .findings
        .retain(|f| !exclude_set.contains(f.control_id.as_ref()));
}

fn apply_only_filter(result: &mut libverify_core::assessment::VerificationResult, only: &[String]) {
    if only.is_empty() {
        return;
    }
    let only_set: std::collections::HashSet<&str> = only.iter().map(String::as_str).collect();
    let known_ids: std::collections::HashSet<String> = result
        .report
        .outcomes
        .iter()
        .map(|o| o.control_id.to_string())
        .collect();
    for o in only {
        if !known_ids.contains(o.as_str()) {
            eprintln!(
                "warning: unknown control ID '{}' in --only (see 'gh verify controls' for valid IDs)",
                o
            );
        }
    }
    result
        .report
        .outcomes
        .retain(|o| only_set.contains(o.control_id.as_ref()));
    result
        .report
        .findings
        .retain(|f| only_set.contains(f.control_id.as_ref()));
}

fn recalculate_batch_totals(batch: &mut libverify_core::assessment::BatchReport) {
    use libverify_core::profile::GateDecision;
    let (mut pass, mut review, mut fail) = (0usize, 0usize, 0usize);
    for entry in &batch.reports {
        for o in &entry.result.report.outcomes {
            match o.decision {
                GateDecision::Pass => pass += 1,
                GateDecision::Review => review += 1,
                GateDecision::Fail => fail += 1,
            }
        }
    }
    batch.total_pass = pass;
    batch.total_review = review;
    batch.total_fail = fail;
}

fn apply_batch_only_filter(batch: &mut libverify_core::assessment::BatchReport, only: &[String]) {
    if only.is_empty() {
        return;
    }
    for entry in &mut batch.reports {
        apply_only_filter(&mut entry.result, only);
    }
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
    let endpoint = format!("repos/{owner}/{repo}");
    let output = run_gh(&["api", endpoint.as_str(), "--silent"]).context(
        "failed to run 'gh api'. Ensure the GitHub CLI (gh) is installed and authenticated",
    )?;
    if !output.status.success() {
        anyhow::bail!(
            "repository '{owner}/{repo}' not found or not accessible. Check the name and your permissions"
        );
    }
    Ok(())
}

fn resolve_repo(cfg: &GitHubConfig, override_repo: Option<&str>) -> Result<(String, String)> {
    let repo_str: String = match override_repo {
        Some(s) if !s.trim().is_empty() => s.to_string(),
        Some(_) => anyhow::bail!("--repo value cannot be empty. Use OWNER/REPO format"),
        _ if !cfg.repo.is_empty() => cfg.repo.clone(),
        _ => detect_repo_from_git_remote()?,
    };
    let slash_idx = repo_str
        .find('/')
        .context("could not resolve repo. Use --repo OWNER/REPO or set GH_REPO env var")?;
    let owner = repo_str[..slash_idx].to_string();
    let repo_name = repo_str[slash_idx + 1..].to_string();
    if owner.is_empty() || repo_name.is_empty() || repo_name.contains('/') {
        anyhow::bail!(
            "invalid repo format '{}'. Expected OWNER/REPO (e.g. cli/cli)",
            repo_str
        );
    }
    Ok((owner, repo_name))
}

fn detect_repo_from_git_remote() -> Result<String> {
    let output = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .context("failed to run 'git remote get-url origin'. Ensure git is installed")?;
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
    let output = run_gh(&["pr", "view", "--json", "number", "--jq", ".number"]).context(
        "failed to run 'gh pr view'. Ensure the GitHub CLI (gh) is installed and authenticated",
    )?;
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

// ---------------------------------------------------------------------------
// Fleet Matrix: multi-repo × multi-policy verification
// ---------------------------------------------------------------------------

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Summary of pass/review/fail counts for a single policy evaluation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PolicySummary {
    pub pass: usize,
    pub review: usize,
    pub fail: usize,
    pub failing_controls: Vec<String>,
}

impl PolicySummary {
    pub fn total(&self) -> usize {
        self.pass + self.review + self.fail
    }
}

/// One row in the fleet matrix: a repo evaluated against multiple policies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetMatrixRow {
    pub repo_id: String,
    pub results: BTreeMap<String, PolicySummary>,
}

/// Fleet matrix: repos × policies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetMatrix {
    pub policies: Vec<String>,
    pub rows: Vec<FleetMatrixRow>,
    /// Per-control worst-across-fleet stats, keyed by control_id then policy.
    pub control_hotspots: Vec<ControlHotspot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlHotspot {
    pub control_id: String,
    /// policy -> number of repos that failed this control
    pub fail_by_policy: BTreeMap<String, usize>,
    pub total_repos: usize,
}

impl FleetMatrix {
    pub fn has_failures(&self) -> bool {
        self.rows
            .iter()
            .any(|row| row.results.values().any(|s| s.fail > 0))
    }
}

fn run_fleet_matrix(
    client: &GitHubClient,
    repo_list: &[(String, String)],
    policies: &[&str],
    reference: &str,
    exclude: &[String],
    only: &[String],
    quiet: bool,
) -> Result<FleetMatrix> {
    use libverify_core::profile::GateDecision;
    use rayon::prelude::*;

    let exclude_set: std::collections::HashSet<&str> = exclude.iter().map(String::as_str).collect();
    let only_set: std::collections::HashSet<&str> = only.iter().map(String::as_str).collect();

    type HotspotMap = BTreeMap<String, BTreeMap<String, usize>>;

    // Parallel evidence collection and assessment per repo
    let repo_results: Vec<Result<(FleetMatrixRow, HotspotMap)>> = repo_list
        .par_iter()
        .map(|(owner, repo_name)| {
            let repo_id = format!("{owner}/{repo_name}");
            if !quiet {
                eprintln!("Collecting evidence for {repo_id}...");
            }

            let bundle = collect_repo_evidence(client, owner, repo_name, reference)
                .with_context(|| format!("failed to collect evidence for {repo_id}"))?;

            let mut results = BTreeMap::new();
            let mut local_hotspots: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();

            for &policy in policies {
                if !quiet {
                    eprintln!("  Assessing {repo_id} with policy: {policy}");
                }

                let mut report = assess_repo_bundle(&bundle, Some(policy), vec![])
                    .with_context(|| format!("failed to assess {repo_id} with policy {policy}"))?;

                if !exclude_set.is_empty() {
                    report
                        .outcomes
                        .retain(|o| !exclude_set.contains(o.control_id.as_ref()));
                }
                if !only_set.is_empty() {
                    report
                        .outcomes
                        .retain(|o| only_set.contains(o.control_id.as_ref()));
                }

                let mut summary = PolicySummary::default();
                for outcome in &report.outcomes {
                    match outcome.decision {
                        GateDecision::Pass => summary.pass += 1,
                        GateDecision::Review => summary.review += 1,
                        GateDecision::Fail => {
                            summary.fail += 1;
                            let cid = outcome.control_id.to_string();
                            summary.failing_controls.push(cid.clone());
                            *local_hotspots
                                .entry(cid)
                                .or_default()
                                .entry(policy.to_string())
                                .or_default() += 1;
                        }
                    }
                }
                results.insert(policy.to_string(), summary);
            }

            Ok((FleetMatrixRow { repo_id, results }, local_hotspots))
        })
        .collect();

    // Merge results preserving input order
    let mut rows = Vec::with_capacity(repo_list.len());
    let mut hotspot_map: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();

    for result in repo_results {
        let (row, local_hotspots) = result?;
        rows.push(row);
        for (control_id, policy_counts) in local_hotspots {
            let entry = hotspot_map.entry(control_id).or_default();
            for (policy, count) in policy_counts {
                *entry.entry(policy).or_default() += count;
            }
        }
    }

    // Build hotspots sorted by total failures descending
    let total_repos = repo_list.len();
    let mut control_hotspots: Vec<ControlHotspot> = hotspot_map
        .into_iter()
        .map(|(control_id, fail_by_policy)| ControlHotspot {
            control_id,
            fail_by_policy,
            total_repos,
        })
        .collect();
    control_hotspots.sort_by(|a, b| {
        let a_total: usize = a.fail_by_policy.values().sum();
        let b_total: usize = b.fail_by_policy.values().sum();
        b_total.cmp(&a_total)
    });

    Ok(FleetMatrix {
        policies: policies.iter().map(|s| s.to_string()).collect(),
        rows,
        control_hotspots,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ssh_github_com() {
        let url = "git@github.com:cli/cli.git";
        assert_eq!(parse_github_remote_url(url), Some("cli/cli".to_string()));
    }

    #[test]
    fn parse_ssh_no_dot_git() {
        let url = "git@github.com:owner/repo";
        assert_eq!(parse_github_remote_url(url), Some("owner/repo".to_string()));
    }

    #[test]
    fn parse_https_github_com() {
        let url = "https://github.com/cli/cli.git";
        assert_eq!(parse_github_remote_url(url), Some("cli/cli".to_string()));
    }

    #[test]
    fn parse_https_no_dot_git() {
        let url = "https://github.com/owner/repo";
        assert_eq!(parse_github_remote_url(url), Some("owner/repo".to_string()));
    }

    #[test]
    fn parse_http_url() {
        let url = "http://github.com/owner/repo.git";
        assert_eq!(parse_github_remote_url(url), Some("owner/repo".to_string()));
    }

    #[test]
    fn parse_extra_path_segments_rejected() {
        let url = "https://github.com/owner/repo/extra";
        assert_eq!(parse_github_remote_url(url), None);
    }

    #[test]
    fn parse_no_slash_rejected() {
        let url = "https://github.com/onlyowner";
        assert_eq!(parse_github_remote_url(url), None);
    }

    #[test]
    fn parse_empty_rejected() {
        assert_eq!(parse_github_remote_url(""), None);
    }

    #[test]
    fn parse_ssh_extra_path_rejected() {
        let url = "git@github.com:owner/repo/extra.git";
        assert_eq!(parse_github_remote_url(url), None);
    }

    #[test]
    fn resolve_repo_rejects_extra_slashes() {
        let cfg = GitHubConfig {
            repo: String::new(),
            host: String::new(),
            token: String::new(),
        };
        let result = resolve_repo(&cfg, Some("owner/repo/extra"));
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("invalid repo format")
        );
    }

    #[test]
    fn resolve_repo_rejects_empty() {
        let cfg = GitHubConfig {
            repo: String::new(),
            host: String::new(),
            token: String::new(),
        };
        let result = resolve_repo(&cfg, Some(""));
        assert!(result.is_err());
    }

    #[test]
    fn resolve_repo_rejects_whitespace() {
        let cfg = GitHubConfig {
            repo: String::new(),
            host: String::new(),
            token: String::new(),
        };
        let result = resolve_repo(&cfg, Some("  "));
        assert!(result.is_err());
    }

    #[test]
    fn resolve_repo_valid() {
        let cfg = GitHubConfig {
            repo: String::new(),
            host: String::new(),
            token: String::new(),
        };
        let result = resolve_repo(&cfg, Some("owner/repo"));
        assert!(result.is_ok());
        let (owner, repo) = result.unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(repo, "repo");
    }

    #[test]
    fn all_controls_have_remediation_hints() {
        use libverify_core::control::builtin_remediation_hint;
        let missing: Vec<&str> = CONTROLS
            .iter()
            .flat_map(|s| s.controls.iter())
            .filter(|(id, _)| builtin_remediation_hint(id).is_none())
            .map(|(id, _)| *id)
            .collect();
        assert!(
            missing.is_empty(),
            "Controls without remediation hints: {:?}",
            missing
        );
    }

    #[test]
    fn parse_ghes_ssh_without_env_returns_none() {
        // GHES SSH remotes require GH_HOST to be set
        let url = "git@ghes.internal.corp:owner/repo.git";
        // Without GH_HOST set to ghes.internal.corp, this should return None
        // (falls through to github.com which doesn't match)
        assert_eq!(parse_github_remote_url(url), None);
    }

    #[test]
    fn parse_ghes_https_without_env_returns_none() {
        let url = "https://ghes.internal.corp/owner/repo.git";
        assert_eq!(parse_github_remote_url(url), None);
    }

    #[test]
    fn controls_data_not_empty() {
        let total: usize = CONTROLS.iter().map(|s| s.controls.len()).sum();
        assert_eq!(total, 34, "Expected 34 controls, found {total}");
    }

    #[test]
    fn policies_data_not_empty() {
        assert_eq!(
            POLICIES.len(),
            15,
            "Expected 15 policies, found {}",
            POLICIES.len()
        );
    }
}

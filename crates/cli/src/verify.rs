use std::process;

use anyhow::{Context, Result, bail};
use serde::Serialize;

use gh_verify_core::assessment::AssessmentReport;
use gh_verify_core::evidence::EvidenceState;
use gh_verify_core::profile::GateDecision;
use gh_verify_core::slsa::SlsaLevel;

use crate::adapters;
use crate::github;
use crate::github::client::GitHubClient;
use crate::github::graphql::PrData;
use crate::github::types::{CombinedStatusResponse, CommitStatusItem};
use crate::policy::OpaProfile;

/// Verify a single pull request and return an assessment report.
pub fn verify_single_pr(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    pr_number: u32,
    policy: Option<&str>,
    slsa_level: Option<&str>,
) -> Result<AssessmentReport> {
    let pr_data = github::graphql::fetch_pr(client, owner, repo, pr_number)
        .context("failed to fetch PR data")?;
    assess_from_pr_data(&pr_data, owner, repo, pr_number, policy, slsa_level)
}

fn assess_from_pr_data(
    pr_data: &PrData,
    owner: &str,
    repo: &str,
    pr_number: u32,
    policy: Option<&str>,
    slsa_level: Option<&str>,
) -> Result<AssessmentReport> {
    let repo_full = format!("{owner}/{repo}");
    let mut bundle = adapters::github::build_pull_request_bundle(
        &repo_full,
        pr_number,
        &pr_data.metadata,
        &pr_data.files,
        &pr_data.reviews,
        &pr_data.commits,
    );

    let combined_status = if pr_data.commit_statuses.is_empty() {
        None
    } else {
        Some(CombinedStatusResponse {
            state: String::new(),
            statuses: pr_data
                .commit_statuses
                .iter()
                .map(|s| CommitStatusItem {
                    context: s.context.clone(),
                    state: s.state.clone(),
                })
                .collect(),
        })
    };
    let evidence =
        adapters::github::map_check_runs_evidence(&pr_data.check_runs, combined_status.as_ref());
    bundle.check_runs = EvidenceState::complete(evidence);

    if let Some(cr_list) = bundle.check_runs.value() {
        let build_platforms = adapters::github::map_build_platform_evidence(cr_list);
        if !build_platforms.is_empty() {
            bundle.build_platform = EvidenceState::complete(build_platforms);
        }
    }

    assess_bundle(&bundle, policy, slsa_level)
}

/// Batch report aggregating results across multiple PRs.
#[derive(Debug, Serialize)]
pub struct BatchReport {
    pub pr_reports: Vec<PrReport>,
    pub total_pass: usize,
    pub total_review: usize,
    pub total_fail: usize,
    pub skipped: Vec<SkippedPr>,
}

#[derive(Debug, Serialize)]
pub struct PrReport {
    pub pr_number: u32,
    pub report: AssessmentReport,
}

#[derive(Debug, Serialize)]
pub struct SkippedPr {
    pub pr_number: u32,
    pub reason: String,
}

/// Verify a batch of PRs and aggregate results.
pub fn verify_pr_batch(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    pr_numbers: &[u32],
    policy: Option<&str>,
    slsa_level: Option<&str>,
) -> Result<BatchReport> {
    let mut pr_reports = Vec::new();
    let mut skipped = Vec::new();
    let mut total_pass = 0usize;
    let mut total_review = 0usize;
    let mut total_fail = 0usize;
    let total = pr_numbers.len();

    let all_data = github::graphql::fetch_prs(client, owner, repo, pr_numbers);

    for (i, (pr_number, result)) in all_data.into_iter().enumerate() {
        eprintln!("Verifying PR #{pr_number} ({}/{})", i + 1, total);

        match result.and_then(|pr_data| {
            assess_from_pr_data(&pr_data, owner, repo, pr_number, policy, slsa_level)
        }) {
            Ok(report) => {
                for outcome in &report.outcomes {
                    match outcome.decision {
                        GateDecision::Pass => total_pass += 1,
                        GateDecision::Review => total_review += 1,
                        GateDecision::Fail => total_fail += 1,
                    }
                }
                pr_reports.push(PrReport { pr_number, report });
            }
            Err(e) => {
                eprintln!("Warning: skipping PR #{pr_number}: {e:#}");
                skipped.push(SkippedPr {
                    pr_number,
                    reason: format!("{e:#}"),
                });
            }
        }
    }

    Ok(BatchReport {
        pr_reports,
        total_pass,
        total_review,
        total_fail,
        skipped,
    })
}


/// Parse a SLSA level string like "source-l3-build-l2" into (source_level, build_level).
pub fn parse_slsa_level(s: &str) -> Result<(SlsaLevel, SlsaLevel)> {
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

pub fn assess_bundle(
    bundle: &gh_verify_core::evidence::EvidenceBundle,
    policy_path: Option<&str>,
    slsa_level: Option<&str>,
) -> Result<AssessmentReport> {
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
                SlsaLevel::L1,
                SlsaLevel::L1,
            )),
        },
    }
}

pub fn exit_if_assessment_fails(report: &AssessmentReport) {
    if report
        .outcomes
        .iter()
        .any(|o| o.decision == GateDecision::Fail)
    {
        process::exit(1);
    }
}

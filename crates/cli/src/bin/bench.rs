use std::collections::HashMap;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::time::Instant;

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use libverify_core::assessment;
use libverify_core::control::Control;
use libverify_core::controls;
use libverify_core::evidence::EvidenceBundle;
use libverify_core::profile::{ControlProfile, GateDecision};
use libverify_core::scope::is_non_code_file;
use libverify_core::verdict::Severity;
use libverify_github::ossinsight::{CollectionRepoRank, OssInsightClient, PullRequestCreator};
use libverify_github::pr_api;
use libverify_github::{GitHubClient, GitHubConfig};
use libverify_policy::OpaProfile;
use serde::Serialize;

#[derive(Parser)]
#[command(
    name = "gh-verify-bench",
    about = "Run or extend the gh-verify benchmark suite"
)]
struct Cli {
    /// Directory containing case JSON files
    #[arg(long, default_value = "benchmarks/cases")]
    cases_dir: String,
    /// Output format (human or json)
    #[arg(long, default_value = "human")]
    format: String,
    /// Policy presets to benchmark (comma-separated or repeated).
    /// Available: default, oss, aiops, soc1, soc2, slsa-l1, slsa-l2, slsa-l3, slsa-l4, or .rego path.
    #[arg(long = "algorithm", value_delimiter = ',', required = true)]
    algorithms: Vec<String>,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Discover real-world benchmark candidates via OSS Insight and GitHub
    CollectRealWorld(CollectRealWorldArgs),
}

#[derive(Args)]
struct CollectRealWorldArgs {
    /// OSS Insight collection ID
    #[arg(long, default_value_t = 10005)]
    collection_id: u64,
    /// OSS Insight ranking period
    #[arg(long, default_value = "past_28_days")]
    period: String,
    /// Number of ranked repositories to inspect
    #[arg(long, default_value_t = 3)]
    repo_limit: usize,
    /// Number of merged PRs to keep for each repository
    #[arg(long, default_value_t = 2)]
    prs_per_repo: usize,
    /// Number of top PR creators to record for each repository
    #[arg(long, default_value_t = 5)]
    creators_per_repo: u32,
    /// Output manifest path
    #[arg(
        long,
        default_value = "benchmarks/discovery/ossinsight-real-world.json"
    )]
    output: String,
}

/// Resolve algorithm name into (controls, profile).
/// Algorithm name is a policy preset name (default, oss, aiops, soc1, soc2, slsa-l1..l4)
/// or a path to a .rego file.
#[allow(clippy::type_complexity)]
fn resolve_algorithm(name: &str) -> Result<(Vec<Box<dyn Control>>, Box<dyn ControlProfile>)> {
    let profile = OpaProfile::from_preset_or_file(name)?;
    Ok((controls::all_controls(), Box::new(profile)))
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct BenchCase {
    id: String,
    description: String,
    repo: String,
    pr_number: u32,
    expected: Severity,
    rationale: String,
    category: String,
    #[serde(default)]
    target_rule: Option<String>,
    #[serde(default)]
    domains_expected: Vec<String>,
    ecosystem: String,
    #[serde(default)]
    notes: Option<String>,
    #[serde(default)]
    source: Option<BenchCaseSource>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
struct BenchCaseSource {
    provider: String,
    #[serde(default)]
    collection_id: Option<u64>,
    #[serde(default)]
    collection_name: Option<String>,
    #[serde(default)]
    selection: Option<String>,
    #[serde(default)]
    discovered_at: Option<String>,
}

#[derive(Debug, Serialize)]
struct CaseResult {
    id: String,
    expected: Severity,
    actual: ActualResult,
    pass: bool,
    target_rule: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
enum ActualResult {
    Pass,
    Warning,
    Error,
    #[serde(rename = "fetch_error")]
    FetchError(String),
}

impl ActualResult {
    fn matches(&self, expected: &Severity) -> bool {
        matches!(
            (self, expected),
            (ActualResult::Pass, Severity::Pass)
                | (ActualResult::Warning, Severity::Warning)
                | (ActualResult::Error, Severity::Error)
        )
    }

    fn as_severity(&self) -> Option<Severity> {
        match self {
            ActualResult::Pass => Some(Severity::Pass),
            ActualResult::Warning => Some(Severity::Warning),
            ActualResult::Error => Some(Severity::Error),
            ActualResult::FetchError(_) => None,
        }
    }
}

#[derive(Debug, Default)]
struct ClassMetrics {
    tp: u32,
    fp: u32,
    r#fn: u32,
}

impl ClassMetrics {
    fn precision(&self) -> Option<f64> {
        let denom = self.tp + self.fp;
        (denom > 0).then(|| self.tp as f64 / denom as f64)
    }

    fn recall(&self) -> Option<f64> {
        let denom = self.tp + self.r#fn;
        (denom > 0).then(|| self.tp as f64 / denom as f64)
    }

    fn f1(&self) -> Option<f64> {
        let p = self.precision()?;
        let r = self.recall()?;
        let denom = p + r;
        (denom > 0.0).then(|| 2.0 * p * r / denom)
    }
}

#[derive(Debug, Serialize)]
struct RuleMetrics {
    total: usize,
    correct: usize,
    accuracy: f64,
    macro_f1: Option<f64>,
}

#[derive(Debug, Serialize)]
struct Report {
    total: usize,
    correct: usize,
    accuracy: f64,
    macro_f1: Option<f64>,
    per_rule: HashMap<String, RuleMetrics>,
    results: Vec<CaseResult>,
}

#[derive(Debug, Serialize)]
struct DiscoveryManifest {
    generated_at: String,
    collection_id: u64,
    period: String,
    repos: Vec<DiscoveryRepo>,
}

#[derive(Debug, Serialize)]
struct DiscoveryRepo {
    repo: String,
    current_period_rank: String,
    current_period_growth: String,
    total_prs: String,
    top_pr_creators: Vec<PullRequestCreator>,
    prs: Vec<DiscoveryPr>,
}

#[derive(Debug, Serialize)]
struct DiscoveryPr {
    number: u32,
    title: String,
    merged_at: String,
    changed_files: usize,
    code_files: usize,
    changed_paths: Vec<String>,
    code_paths: Vec<String>,
    observed: ActualResult,
    source: BenchCaseSource,
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
        Some(Command::CollectRealWorld(args)) => collect_real_world(args),
        None => run_benchmarks(&cli),
    }
}

fn fetch_evidence(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    pr_number: u32,
) -> Result<EvidenceBundle> {
    let pr_files = pr_api::get_pr_files(client, owner, repo, pr_number)?;
    let pr_metadata = pr_api::get_pr_metadata(client, owner, repo, pr_number)?;
    let pr_reviews = pr_api::get_pr_reviews(client, owner, repo, pr_number)?;
    let pr_commits = pr_api::get_pr_commits(client, owner, repo, pr_number)?;

    let repo_full = format!("{owner}/{repo}");
    Ok(libverify_github::adapter::build_pull_request_bundle(
        &repo_full,
        pr_number,
        &pr_metadata,
        &pr_files,
        &pr_reviews,
        &pr_commits,
    ))
}

fn decision_to_result(d: GateDecision) -> ActualResult {
    match d {
        GateDecision::Pass => ActualResult::Pass,
        GateDecision::Review => ActualResult::Warning,
        GateDecision::Fail => ActualResult::Error,
    }
}

fn decision_rank(d: &GateDecision) -> u8 {
    match d {
        GateDecision::Pass => 0,
        GateDecision::Review => 1,
        GateDecision::Fail => 2,
    }
}

fn assess_bundle(
    bundle: &EvidenceBundle,
    controls: &[Box<dyn Control>],
    profile: &dyn ControlProfile,
    target_rule: Option<&str>,
) -> ActualResult {
    let report = assessment::assess(bundle, controls, profile);

    // When target_rule is specified, use only that control's outcome.
    if let Some(rule) = target_rule {
        if let Some(outcome) = report
            .outcomes
            .iter()
            .find(|o| o.control_id.as_str() == rule)
        {
            return decision_to_result(outcome.decision);
        }
        // Control not found in outcomes — treat as pass (control was not applicable).
        return ActualResult::Pass;
    }

    // Fallback: max severity across all controls.
    let max_decision = report
        .outcomes
        .iter()
        .map(|o| o.decision)
        .max_by_key(|d| decision_rank(d))
        .unwrap_or(GateDecision::Pass);

    decision_to_result(max_decision)
}

fn run_benchmarks(cli: &Cli) -> Result<()> {
    let dir = PathBuf::from(&cli.cases_dir);
    let cases = load_cases(&dir)?;
    if cases.is_empty() {
        anyhow::bail!("no benchmark cases found in {}", dir.display());
    }

    // Validate all algorithm names upfront.
    for name in &cli.algorithms {
        resolve_algorithm(name)?;
    }

    let cfg = GitHubConfig::load()?;
    let client = GitHubClient::new(&cfg)?;

    eprintln!("ghverify benchmark");
    eprintln!("==================");
    eprintln!("Algorithms: {}", cli.algorithms.join(", "));
    eprintln!();

    let total_cases = cases.len();
    let bench_started = Instant::now();
    let mut algo_reports: Vec<(String, Report)> = Vec::new();

    for algo_name in &cli.algorithms {
        let (controls, profile) = resolve_algorithm(algo_name)?;

        eprintln!("--- Algorithm: {algo_name} ---");
        eprintln!();

        let mut results = Vec::with_capacity(cases.len());
        let mut correct_so_far = 0usize;

        for (idx, case) in cases.iter().enumerate() {
            let case_no = idx + 1;
            eprintln!(
                "[{case_no}/{total_cases}] RUN  {:<12} | {:<30} #{:<5} | expected={}",
                case.id,
                case.repo,
                case.pr_number,
                sev_str(&case.expected)
            );
            let _ = io::stderr().flush();

            let case_started = Instant::now();
            let actual = run_case_with(&client, case, &controls, profile.as_ref());
            let pass = actual.matches(&case.expected);
            let case_secs = case_started.elapsed().as_secs_f32();

            if pass {
                correct_so_far += 1;
            }
            let progress_accuracy = (correct_so_far as f64 / case_no as f64) * 100.0;

            if pass {
                eprintln!(
                    "\x1b[32m[PASS]\x1b[0m {:<12} | {:.2}s | progress={}/{} ({:.1}%)",
                    case.id, case_secs, correct_so_far, case_no, progress_accuracy
                );
            } else {
                eprintln!(
                    "\x1b[31m[FAIL]\x1b[0m {:<12} | {:.2}s | progress={}/{} ({:.1}%) | expected={:<7} actual={}",
                    case.id,
                    case_secs,
                    correct_so_far,
                    case_no,
                    progress_accuracy,
                    sev_str(&case.expected),
                    actual_str(&actual)
                );
            }

            let target = case
                .target_rule
                .as_deref()
                .unwrap_or("assessment")
                .to_owned();
            results.push(CaseResult {
                id: case.id.clone(),
                expected: case.expected,
                actual,
                pass,
                target_rule: target,
            });
        }

        let report = build_report(results);
        print_report(algo_name, &report);
        algo_reports.push((algo_name.clone(), report));
        eprintln!();
    }

    eprintln!("=== Comparison ===");
    eprintln!();
    eprintln!(
        "{:<20} {:>6} {:>8} {:>10} {:>10}",
        "Algorithm", "Total", "Correct", "Accuracy", "Macro F1"
    );
    eprintln!(
        "{:<20} {:>6} {:>8} {:>10} {:>10}",
        "----", "-----", "-------", "--------", "--------"
    );
    for (name, report) in &algo_reports {
        let f1_str = report
            .macro_f1
            .map(|v| format!("{v:.4}"))
            .unwrap_or_else(|| "N/A".into());
        eprintln!(
            "{:<20} {:>6} {:>8} {:>9.1}% {:>10}",
            name,
            report.total,
            report.correct,
            report.accuracy * 100.0,
            f1_str
        );
    }

    eprintln!();
    eprintln!(
        "Total elapsed: {:.2}s",
        bench_started.elapsed().as_secs_f32()
    );

    if cli.format == "json" {
        let json_map: HashMap<String, &Report> = algo_reports
            .iter()
            .map(|(name, report)| (name.clone(), report))
            .collect();
        println!("{}", serde_json::to_string_pretty(&json_map)?);
    }

    Ok(())
}

fn print_report(algo_name: &str, report: &Report) {
    eprintln!();
    eprintln!(
        "[{algo_name}] Accuracy: {}/{} ({:.1}%)",
        report.correct,
        report.total,
        report.accuracy * 100.0
    );
    if let Some(f1) = report.macro_f1 {
        eprintln!("[{algo_name}] Macro F1: {f1:.4}");
    } else {
        eprintln!("[{algo_name}] Macro F1: N/A");
    }
    eprintln!();

    let metrics = compute_metrics(&report.results);
    eprintln!(
        "{:<10} {:>4} {:>4} {:>4} {:>10} {:>10} {:>10}",
        "Severity", "TP", "FP", "FN", "Precision", "Recall", "F1"
    );
    eprintln!(
        "{:<10} {:>4} {:>4} {:>4} {:>10} {:>10} {:>10}",
        "--------", "--", "--", "--", "---------", "------", "--"
    );
    for s in [Severity::Pass, Severity::Warning, Severity::Error] {
        let default = ClassMetrics::default();
        let m = metrics.get(&s).unwrap_or(&default);
        let p = m
            .precision()
            .map(|v| format!("{:.1}%", v * 100.0))
            .unwrap_or_else(|| "N/A".into());
        let r = m
            .recall()
            .map(|v| format!("{:.1}%", v * 100.0))
            .unwrap_or_else(|| "N/A".into());
        let f = m
            .f1()
            .map(|v| format!("{v:.4}"))
            .unwrap_or_else(|| "N/A".into());
        eprintln!(
            "{:<10} {:>4} {:>4} {:>4} {:>10} {:>10} {:>10}",
            sev_str(&s),
            m.tp,
            m.fp,
            m.r#fn,
            p,
            r,
            f
        );
    }

    if report.per_rule.len() > 1 {
        eprintln!();
        eprintln!("Per-rule breakdown:");
        eprintln!(
            "{:<30} {:>6} {:>8} {:>10} {:>10}",
            "Rule", "Total", "Correct", "Accuracy", "Macro F1"
        );
        eprintln!(
            "{:<30} {:>6} {:>8} {:>10} {:>10}",
            "----", "-----", "-------", "--------", "--------"
        );
        let mut rules: Vec<_> = report.per_rule.iter().collect();
        rules.sort_by(|(a, _), (b, _)| a.cmp(b));
        for (rule, m) in &rules {
            let f1_str = m
                .macro_f1
                .map(|v| format!("{v:.4}"))
                .unwrap_or_else(|| "N/A".into());
            eprintln!(
                "{:<30} {:>6} {:>8} {:>9.1}% {:>10}",
                rule,
                m.total,
                m.correct,
                m.accuracy * 100.0,
                f1_str
            );
        }
    }
}

fn collect_real_world(args: CollectRealWorldArgs) -> Result<()> {
    let cfg = GitHubConfig::load()?;
    let github = GitHubClient::new(&cfg)?;
    let ossinsight = OssInsightClient::new()?;

    let ranked_repos = ossinsight.ranking_by_prs(args.collection_id, &args.period)?;
    if ranked_repos.is_empty() {
        anyhow::bail!("OSS Insight returned no ranked repositories");
    }

    let mut repos = Vec::new();
    for rank in ranked_repos.into_iter().take(args.repo_limit) {
        repos.push(discover_repo(
            &github,
            &ossinsight,
            &rank,
            args.collection_id,
            &args.period,
            args.creators_per_repo,
            args.prs_per_repo,
        )?);
    }

    let manifest = DiscoveryManifest {
        generated_at: timestamp_now(),
        collection_id: args.collection_id,
        period: args.period,
        repos,
    };

    write_pretty_json(&args.output, &manifest)?;
    eprintln!(
        "saved {} repositories to {}",
        manifest.repos.len(),
        args.output
    );
    Ok(())
}

fn discover_repo(
    github: &GitHubClient,
    ossinsight: &OssInsightClient,
    rank: &CollectionRepoRank,
    collection_id: u64,
    period: &str,
    creators_per_repo: u32,
    prs_per_repo: usize,
) -> Result<DiscoveryRepo> {
    let (owner, repo) = rank
        .repo_name
        .split_once('/')
        .ok_or_else(|| anyhow::anyhow!("invalid repo name from OSS Insight: {}", rank.repo_name))?;

    let top_pr_creators = ossinsight.pull_request_creators(owner, repo, creators_per_repo)?;
    let merged_prs = pr_api::list_recent_merged_prs(github, owner, repo, prs_per_repo)?;
    let mut prs = Vec::new();

    for pr in merged_prs {
        let files = pr_api::get_pr_files(github, owner, repo, pr.number)?;
        let changed_paths: Vec<String> = files.iter().map(|file| file.filename.clone()).collect();
        let code_paths: Vec<String> = files
            .iter()
            .filter(|file| file.patch.is_some() && !is_non_code_file(&file.filename))
            .map(|file| file.filename.clone())
            .collect();
        let code_files = code_paths.len();
        let observed = match fetch_evidence(github, owner, repo, pr.number) {
            Ok(bundle) => {
                let ctrls = controls::all_controls();
                let profile = OpaProfile::slsa_l1_preset()?;
                assess_bundle(&bundle, &ctrls, &profile, None)
            }
            Err(e) => ActualResult::FetchError(format!("{e}")),
        };

        prs.push(DiscoveryPr {
            number: pr.number,
            title: pr.title,
            merged_at: pr.merged_at.unwrap_or_default(),
            changed_files: files.len(),
            code_files,
            changed_paths,
            code_paths,
            observed,
            source: BenchCaseSource {
                provider: "ossinsight".into(),
                collection_id: Some(collection_id),
                collection_name: None,
                selection: Some(format!(
                    "ranking_by_prs(period={period}) + recent merged PRs"
                )),
                discovered_at: Some(timestamp_now()),
            },
        });
    }

    Ok(DiscoveryRepo {
        repo: rank.repo_name.clone(),
        current_period_rank: rank.current_period_rank.clone(),
        current_period_growth: rank.current_period_growth.clone(),
        total_prs: rank.total.clone(),
        top_pr_creators,
        prs,
    })
}

fn run_case_with(
    client: &GitHubClient,
    case: &BenchCase,
    controls: &[Box<dyn Control>],
    profile: &dyn ControlProfile,
) -> ActualResult {
    let (owner, repo) = match case.repo.split_once('/') {
        Some(pair) => pair,
        None => return ActualResult::FetchError("invalid repo format".into()),
    };

    let target_rule = case.target_rule.as_deref();

    match fetch_evidence(client, owner, repo, case.pr_number) {
        Ok(bundle) => assess_bundle(&bundle, controls, profile, target_rule),
        Err(e) => ActualResult::FetchError(format!("{e}")),
    }
}

fn compute_macro_f1(results: &[CaseResult]) -> Option<f64> {
    let metrics = compute_metrics(results);
    let f1_values: Vec<f64> = [Severity::Pass, Severity::Warning, Severity::Error]
        .iter()
        .filter_map(|s| metrics.get(s).and_then(|m| m.f1()))
        .collect();
    if f1_values.is_empty() {
        None
    } else {
        Some(f1_values.iter().sum::<f64>() / f1_values.len() as f64)
    }
}

fn build_report(results: Vec<CaseResult>) -> Report {
    let total = results.len();
    let correct = results.iter().filter(|r| r.pass).count();
    let accuracy = if total > 0 {
        correct as f64 / total as f64
    } else {
        0.0
    };
    let macro_f1 = compute_macro_f1(&results);

    let mut per_rule_cases: HashMap<String, Vec<&CaseResult>> = HashMap::new();
    for r in &results {
        per_rule_cases
            .entry(r.target_rule.clone())
            .or_default()
            .push(r);
    }
    let mut per_rule = HashMap::new();
    for (rule, cases) in &per_rule_cases {
        let rule_total = cases.len();
        let rule_correct = cases.iter().filter(|r| r.pass).count();
        let rule_accuracy = if rule_total > 0 {
            rule_correct as f64 / rule_total as f64
        } else {
            0.0
        };
        let owned: Vec<CaseResult> = cases
            .iter()
            .map(|r| CaseResult {
                id: r.id.clone(),
                expected: r.expected,
                actual: r.actual.clone(),
                pass: r.pass,
                target_rule: r.target_rule.clone(),
            })
            .collect();
        let rule_macro_f1 = compute_macro_f1(&owned);
        per_rule.insert(
            rule.clone(),
            RuleMetrics {
                total: rule_total,
                correct: rule_correct,
                accuracy: rule_accuracy,
                macro_f1: rule_macro_f1,
            },
        );
    }

    Report {
        total,
        correct,
        accuracy,
        macro_f1,
        per_rule,
        results,
    }
}

fn compute_metrics(results: &[CaseResult]) -> HashMap<Severity, ClassMetrics> {
    let mut metrics: HashMap<Severity, ClassMetrics> = HashMap::new();
    for s in [Severity::Pass, Severity::Warning, Severity::Error] {
        metrics.insert(s, ClassMetrics::default());
    }

    for r in results {
        if r.pass {
            metrics.get_mut(&r.expected).unwrap().tp += 1;
        } else {
            metrics.get_mut(&r.expected).unwrap().r#fn += 1;
            if let Some(actual_sev) = r.actual.as_severity() {
                metrics.get_mut(&actual_sev).unwrap().fp += 1;
            }
        }
    }
    metrics
}

fn load_cases(dir: &Path) -> Result<Vec<BenchCase>> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .with_context(|| format!("cannot read benchmark cases dir: {}", dir.display()))?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    let mut cases = Vec::new();
    for entry in entries {
        let path = entry.path();
        let content = std::fs::read_to_string(&path)?;
        let case: BenchCase = serde_json::from_str(&content)
            .with_context(|| format!("invalid case: {}", path.display()))?;
        cases.push(case);
    }
    Ok(cases)
}

fn write_pretty_json<T: serde::Serialize>(path: impl Into<PathBuf>, value: &T) -> Result<()> {
    let path = path.into();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("cannot create dir: {}", parent.display()))?;
    }

    let json = serde_json::to_string_pretty(value)?;
    std::fs::write(&path, format!("{json}\n"))
        .with_context(|| format!("cannot write json file: {}", path.display()))?;
    Ok(())
}

fn sev_str(s: &Severity) -> &'static str {
    match s {
        Severity::Pass => "pass",
        Severity::Warning => "warning",
        Severity::Error => "error",
    }
}

fn actual_str(a: &ActualResult) -> String {
    match a {
        ActualResult::Pass => "pass".into(),
        ActualResult::Warning => "warning".into(),
        ActualResult::Error => "error".into(),
        ActualResult::FetchError(e) => format!("fetch_error({e})"),
    }
}

fn timestamp_now() -> String {
    let output = process::Command::new("date")
        .arg("-u")
        .arg("+%Y-%m-%dT%H:%M:%SZ")
        .output();
    match output {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
        _ => "unknown".into(),
    }
}

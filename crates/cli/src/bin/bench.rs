use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process;

use anyhow::{Context, Result};
use clap::Parser;
use gh_verify_core::verdict::Severity;
use serde::{Deserialize, Serialize};

use gh_verify::config::Config;
use gh_verify::github::client::GitHubClient;
use gh_verify::github::pr_api;
use gh_verify::rules::{self, RuleContext};

#[derive(Parser)]
#[command(name = "gh-verify-bench", about = "Run gh-verify benchmark suite")]
struct Cli {
    /// Directory containing case JSON files
    #[arg(long, default_value = "benchmarks/cases")]
    cases_dir: String,
    /// Output format (human or json)
    #[arg(long, default_value = "human")]
    format: String,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e:#}");
        process::exit(1);
    }
}

/// A benchmark case read from JSON.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct BenchCase {
    id: String,
    description: String,
    repo: String,
    pr_number: u32,
    expected: Severity,
    category: String,
    ecosystem: String,
}

#[derive(Debug, Serialize)]
struct CaseResult {
    id: String,
    expected: Severity,
    actual: ActualResult,
    pass: bool,
}

#[derive(Debug, Serialize)]
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
struct Report {
    total: usize,
    correct: usize,
    accuracy: f64,
    macro_f1: Option<f64>,
    results: Vec<CaseResult>,
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let dir = PathBuf::from(&cli.cases_dir);

    let cases = load_cases(&dir)?;
    if cases.is_empty() {
        anyhow::bail!("no benchmark cases found in {}", dir.display());
    }

    let cfg = Config::load()?;
    let client = GitHubClient::new(&cfg)?;

    eprintln!("ghverify benchmark");
    eprintln!("==================");
    eprintln!();

    let mut results = Vec::with_capacity(cases.len());

    for case in &cases {
        let actual = run_case(&client, case);
        let pass = actual.matches(&case.expected);

        if pass {
            eprintln!(
                "\x1b[32m[PASS]\x1b[0m {:<12} | {:<30} #{:<5} | expected={}",
                case.id, case.repo, case.pr_number, sev_str(&case.expected)
            );
        } else {
            eprintln!(
                "\x1b[31m[FAIL]\x1b[0m {:<12} | {:<30} #{:<5} | expected={:<7} actual={}",
                case.id,
                case.repo,
                case.pr_number,
                sev_str(&case.expected),
                actual_str(&actual)
            );
        }

        results.push(CaseResult {
            id: case.id.clone(),
            expected: case.expected,
            actual,
            pass,
        });
    }

    let total = results.len();
    let correct = results.iter().filter(|r| r.pass).count();
    let accuracy = if total > 0 {
        correct as f64 / total as f64
    } else {
        0.0
    };

    let metrics = compute_metrics(&results);
    let f1_values: Vec<f64> = [Severity::Pass, Severity::Warning, Severity::Error]
        .iter()
        .filter_map(|s| metrics.get(s).and_then(|m| m.f1()))
        .collect();
    let macro_f1 = if f1_values.is_empty() {
        None
    } else {
        Some(f1_values.iter().sum::<f64>() / f1_values.len() as f64)
    };

    eprintln!();
    eprintln!("Accuracy: {correct}/{total} ({:.1}%)", accuracy * 100.0);
    if let Some(f1) = macro_f1 {
        eprintln!("Macro F1: {f1:.4}");
    } else {
        eprintln!("Macro F1: N/A");
    }
    eprintln!();
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

    if cli.format == "json" {
        let report = Report {
            total,
            correct,
            accuracy,
            macro_f1,
            results,
        };
        println!("{}", serde_json::to_string_pretty(&report)?);
    }

    Ok(())
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
        let content = std::fs::read_to_string(entry.path())?;
        let case: BenchCase = serde_json::from_str(&content)
            .with_context(|| format!("invalid case: {}", entry.path().display()))?;
        cases.push(case);
    }
    Ok(cases)
}

fn run_case(client: &GitHubClient, case: &BenchCase) -> ActualResult {
    let (owner, repo) = match case.repo.split_once('/') {
        Some(pair) => pair,
        None => return ActualResult::FetchError("invalid repo format".into()),
    };

    let pr_files = match pr_api::get_pr_files(client, owner, repo, case.pr_number) {
        Ok(f) => f,
        Err(e) => return ActualResult::FetchError(format!("files: {e}")),
    };

    let pr_metadata = match pr_api::get_pr_metadata(client, owner, repo, case.pr_number) {
        Ok(m) => m,
        Err(e) => return ActualResult::FetchError(format!("metadata: {e}")),
    };

    let ctx = RuleContext::Pr {
        pr_files,
        pr_metadata,
    };

    match rules::engine::run_all(&ctx) {
        Ok(results) => {
            if results.is_empty() {
                return ActualResult::Pass;
            }
            let max_sev = results
                .iter()
                .map(|r| r.severity)
                .max()
                .unwrap_or(Severity::Pass);
            match max_sev {
                Severity::Pass => ActualResult::Pass,
                Severity::Warning => ActualResult::Warning,
                Severity::Error => ActualResult::Error,
            }
        }
        Err(e) => ActualResult::FetchError(format!("engine: {e}")),
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

use anyhow::Result;
use gh_verify_core::scope::{FileRole, classify_file_role, is_non_code_file};
use gh_verify_core::test_coverage::has_test_coverage;
use gh_verify_core::verdict::{RuleResult, Severity};

use crate::github::types::PrFile;

use super::{PrRuleOptions, Rule, RuleContext};

const RULE_ID: &str = "detect-missing-test";

pub struct DetectMissingTest;

impl DetectMissingTest {
    /// Tier 1: Coverage-report-based verification.
    /// Uses LCOV data to check whether changed lines are actually executed by tests.
    fn run_with_coverage(
        &self,
        pr_files: &[PrFile],
        lcov_content: &str,
    ) -> Result<Vec<RuleResult>> {
        use gh_verify_core::coverage;

        let report = coverage::parse_lcov(lcov_content)
            .map_err(|e| anyhow::anyhow!("LCOV parse error: {e:?}"))?;

        let changed_files: Vec<(String, Vec<u32>)> = pr_files
            .iter()
            .filter(|f| f.status != "removed")
            .filter(|f| !is_non_code_file(&f.filename))
            .filter(|f| classify_file_role(&f.filename) == FileRole::Source)
            .filter_map(|f| {
                f.patch.as_deref().map(|patch| {
                    (f.filename.clone(), coverage::extract_changed_lines(patch))
                })
            })
            .filter(|(_, lines)| !lines.is_empty())
            .collect();

        if changed_files.is_empty() {
            return Ok(vec![pass_result()]);
        }

        let analysis = coverage::analyze_coverage(&report, &changed_files);
        let severity = coverage::classify_coverage_severity(
            analysis.total_covered as usize,
            analysis.total_changed as usize,
            80, 50, // warn=80%, error=50%
        );

        let affected: Vec<String> = analysis
            .files
            .iter()
            .filter(|f| f.coverage_pct < 80.0)
            .map(|f| f.path.clone())
            .collect();

        let suggestion = if affected.is_empty() {
            None
        } else {
            let details: Vec<String> = analysis
                .files
                .iter()
                .filter(|f| f.coverage_pct < 80.0)
                .map(|f| {
                    format!(
                        "  {} ({:.0}%, uncovered: {:?})",
                        f.path, f.coverage_pct, f.uncovered_line_numbers
                    )
                })
                .collect();
            Some(format!("Low coverage files:\n{}", details.join("\n")))
        };

        Ok(vec![RuleResult {
            rule_id: RULE_ID.to_string(),
            severity,
            message: format!(
                "Changed code coverage: {:.0}% ({}/{} lines)",
                analysis.overall_pct, analysis.total_covered, analysis.total_changed
            ),
            affected_files: affected,
            suggestion,
        }])
    }

    /// Tier 2: Heuristic-based fallback.
    /// Checks whether source files have corresponding test file changes in the PR.
    fn run_heuristic(
        &self,
        pr_files: &[PrFile],
        options: &PrRuleOptions,
    ) -> Result<Vec<RuleResult>> {
        let changed_code_files: Vec<&str> = pr_files
            .iter()
            .filter(|f| f.patch.is_some())
            .filter(|f| f.status != "removed")
            .filter(|f| !is_non_code_file(&f.filename))
            .map(|f| f.filename.as_str())
            .collect();

        let source_files: Vec<&str> = changed_code_files
            .iter()
            .copied()
            .filter(|p| classify_file_role(p) == FileRole::Source)
            .collect();
        let test_files: Vec<&str> = changed_code_files
            .iter()
            .copied()
            .filter(|p| classify_file_role(p) == FileRole::Test)
            .collect();

        if source_files.is_empty() {
            return Ok(vec![pass_result()]);
        }

        let mut uncovered = has_test_coverage(&source_files, &test_files);
        if !options.test_patterns.is_empty() {
            uncovered.retain(|entry| {
                !is_covered_by_custom_patterns(
                    &entry.source_path,
                    &test_files,
                    &options.test_patterns,
                )
            });
        }

        if uncovered.is_empty() {
            return Ok(vec![pass_result()]);
        }

        let affected_files: Vec<String> =
            uncovered.iter().map(|u| u.source_path.clone()).collect();
        let mut suggestion_lines = Vec::new();
        for entry in &uncovered {
            let first_candidate = entry
                .suggested_test_paths
                .first()
                .cloned()
                .unwrap_or_else(|| "(候補なし)".to_string());
            suggestion_lines.push(format!("{} -> {}", entry.source_path, first_candidate));
        }

        Ok(vec![RuleResult {
            rule_id: RULE_ID.to_string(),
            severity: Severity::Warning,
            message: format!(
                "{} source file(s) changed without matching test updates",
                uncovered.len()
            ),
            affected_files,
            suggestion: Some(format!(
                "Consider updating tests for:\n{}",
                suggestion_lines.join("\n")
            )),
        }])
    }
}

impl Rule for DetectMissingTest {
    fn id(&self) -> &'static str {
        RULE_ID
    }

    fn run(&self, ctx: &RuleContext) -> Result<Vec<RuleResult>> {
        let (pr_files, options) = match ctx {
            RuleContext::Pr {
                pr_files, options, ..
            } => (pr_files, options),
            RuleContext::Release { .. } => return Ok(vec![]),
        };

        if !options.detect_missing_test {
            return Ok(vec![]);
        }

        // Tier 1: Coverage report available — verify changed lines are tested
        if let Some(ref lcov_content) = options.coverage_report {
            return self.run_with_coverage(pr_files, lcov_content);
        }

        // Tier 2: Fallback to existing heuristic (file naming convention)
        self.run_heuristic(pr_files, options)
    }
}

fn pass_result() -> RuleResult {
    RuleResult::pass(RULE_ID, "Source changes include matching test updates")
}

fn is_covered_by_custom_patterns(
    source_path: &str,
    test_files: &[&str],
    patterns: &[String],
) -> bool {
    let source_stem = normalized_file_stem(source_path);
    if source_stem.is_empty() {
        return false;
    }

    patterns.iter().any(|pattern| {
        let candidate = pattern.replace('*', &source_stem);
        test_files.iter().any(|test| test.ends_with(&candidate))
    })
}

fn normalized_file_stem(path: &str) -> String {
    let file = path.rsplit('/').next().unwrap_or(path);
    let stem = file.split('.').next().unwrap_or(file);
    stem.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::types::{PrFile, PrMetadata};
    use crate::rules::{PrRuleOptions, RuleContext};

    fn make_ctx(files: Vec<PrFile>, options: PrRuleOptions) -> RuleContext {
        RuleContext::Pr {
            pr_files: files,
            pr_metadata: metadata(),
            pr_reviews: vec![],
            pr_commits: vec![],
            options,
        }
    }

    fn make_ctx_with_coverage(files: Vec<PrFile>, lcov: &str) -> RuleContext {
        RuleContext::Pr {
            pr_files: files,
            pr_metadata: metadata(),
            pr_reviews: vec![],
            pr_commits: vec![],
            options: PrRuleOptions {
                detect_missing_test: true,
                test_patterns: vec![],
                coverage_report: Some(lcov.to_string()),
            },
        }
    }

    // --- Existing tests (Tier 2 heuristic) ---

    #[test]
    fn warns_when_source_has_no_test_update() {
        let ctx = make_ctx(
            vec![file("src/foo.rs"), file("README.md")],
            PrRuleOptions::default(),
        );

        let results = DetectMissingTest.run(&ctx).expect("rule should run");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Warning);
        assert_eq!(results[0].affected_files, vec!["src/foo.rs".to_string()]);
    }

    #[test]
    fn passes_when_test_pair_is_changed() {
        let ctx = make_ctx(
            vec![file("src/foo.rs"), file("tests/foo_test.rs")],
            PrRuleOptions::default(),
        );

        let results = DetectMissingTest.run(&ctx).expect("rule should run");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Pass);
    }

    #[test]
    fn can_disable_rule_with_option() {
        let ctx = make_ctx(
            vec![file("src/foo.rs")],
            PrRuleOptions {
                detect_missing_test: false,
                test_patterns: vec![],
                coverage_report: None,
            },
        );

        let results = DetectMissingTest.run(&ctx).expect("rule should run");
        assert!(results.is_empty());
    }

    #[test]
    fn custom_pattern_covers_source() {
        let ctx = make_ctx(
            vec![file("src/foo.rs"), file("spec/foo.spec.rs")],
            PrRuleOptions {
                detect_missing_test: true,
                test_patterns: vec!["*.spec.rs".to_string()],
                coverage_report: None,
            },
        );

        let results = DetectMissingTest.run(&ctx).expect("rule should run");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Pass);
    }

    // --- New tests (Tier 1 coverage) ---

    #[test]
    /// WHY: Tier 1 must return Pass when coverage report shows all changed lines
    /// are covered. This confirms the happy path through the coverage pipeline.
    fn coverage_report_high_coverage_passes() {
        let lcov = "\
SF:src/foo.rs
DA:1,1
DA:2,1
DA:3,1
LF:3
LH:3
end_of_record
";
        let files = vec![file_with_patch(
            "src/foo.rs",
            "@@ -0,0 +1,3 @@\n+line1\n+line2\n+line3\n",
        )];
        let ctx = make_ctx_with_coverage(files, lcov);

        let results = DetectMissingTest.run(&ctx).expect("rule should run");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Pass);
        assert!(results[0].message.contains("100"));
    }

    #[test]
    /// WHY: Tier 1 can emit Error severity, unlike Tier 2 which caps at Warning.
    /// This is the key differentiation: coverage data provides enough confidence
    /// to block PRs with dangerously low coverage.
    fn coverage_report_low_coverage_errors() {
        let lcov = "\
SF:src/foo.rs
DA:1,0
DA:2,0
DA:3,0
LF:3
LH:0
end_of_record
";
        let files = vec![file_with_patch(
            "src/foo.rs",
            "@@ -0,0 +1,3 @@\n+line1\n+line2\n+line3\n",
        )];
        let ctx = make_ctx_with_coverage(files, lcov);

        let results = DetectMissingTest.run(&ctx).expect("rule should run");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Error);
        assert!(results[0].affected_files.contains(&"src/foo.rs".to_string()));
    }

    #[test]
    /// WHY: LCOV files from CI often contain absolute paths (e.g. /home/runner/work/proj/src/foo.rs)
    /// while PR file paths are relative (src/foo.rs). The coverage pipeline must resolve
    /// these paths correctly via suffix matching.
    fn coverage_report_path_normalization() {
        let lcov = "\
SF:/home/runner/work/project/src/foo.rs
DA:1,1
DA:2,1
LF:2
LH:2
end_of_record
";
        let files = vec![file_with_patch(
            "src/foo.rs",
            "@@ -0,0 +1,2 @@\n+line1\n+line2\n",
        )];
        let ctx = make_ctx_with_coverage(files, lcov);

        let results = DetectMissingTest.run(&ctx).expect("rule should run");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Pass);
    }

    #[test]
    /// WHY: When no coverage report is provided, the rule must fall back to Tier 2
    /// heuristic behavior (file naming convention). This ensures backward compatibility.
    fn no_coverage_falls_back_to_heuristic() {
        let ctx = make_ctx(
            vec![file("src/foo.rs")],
            PrRuleOptions {
                detect_missing_test: true,
                test_patterns: vec![],
                coverage_report: None,
            },
        );

        let results = DetectMissingTest.run(&ctx).expect("rule should run");
        assert_eq!(results.len(), 1);
        // WHY: Tier 2 heuristic only emits Warning, never Error
        assert_eq!(results[0].severity, Severity::Warning);
    }

    #[test]
    /// WHY: Removed files have no lines to cover — including them would produce
    /// false negatives (0% coverage on files that no longer exist).
    fn coverage_report_with_removed_files() {
        let lcov = "\
SF:src/kept.rs
DA:1,1
LF:1
LH:1
end_of_record
";
        let files = vec![
            file_with_patch("src/kept.rs", "@@ -0,0 +1 @@\n+line1\n"),
            PrFile {
                filename: "src/deleted.rs".to_string(),
                status: "removed".to_string(),
                additions: 0,
                deletions: 10,
                changes: 10,
                patch: None,
            },
        ];
        let ctx = make_ctx_with_coverage(files, lcov);

        let results = DetectMissingTest.run(&ctx).expect("rule should run");
        assert_eq!(results.len(), 1);
        // WHY: Only src/kept.rs should be analyzed; deleted.rs must be excluded
        assert_eq!(results[0].severity, Severity::Pass);
        assert!(
            !results[0]
                .affected_files
                .contains(&"src/deleted.rs".to_string())
        );
    }

    #[test]
    /// WHY: Test/fixture-only PRs have no source files to cover. Without the
    /// FileRole::Source filter, run_with_coverage would see 0 changed lines
    /// mapped to coverage and report 0% → Error. This must be Pass.
    fn coverage_report_test_only_pr_passes() {
        let lcov = "\
SF:tests/foo_test.rs
DA:1,1
LF:1
LH:1
end_of_record
";
        let files = vec![file_with_patch(
            "tests/foo_test.rs",
            "@@ -0,0 +1,2 @@\n+#[test]\n+fn it_works() {}\n",
        )];
        let ctx = make_ctx_with_coverage(files, lcov);

        let results = DetectMissingTest.run(&ctx).expect("rule should run");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Pass);
    }

    // --- Test helpers ---

    fn file(path: &str) -> PrFile {
        PrFile {
            filename: path.to_string(),
            status: "modified".to_string(),
            additions: 1,
            deletions: 1,
            changes: 2,
            patch: Some("@@ -1 +1 @@".to_string()),
        }
    }

    fn file_with_patch(path: &str, patch: &str) -> PrFile {
        PrFile {
            filename: path.to_string(),
            status: "modified".to_string(),
            additions: 1,
            deletions: 0,
            changes: 1,
            patch: Some(patch.to_string()),
        }
    }

    fn metadata() -> PrMetadata {
        PrMetadata {
            number: 1,
            title: "test".to_string(),
            body: None,
        }
    }
}

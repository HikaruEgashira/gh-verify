use anyhow::Result;
use colored::Colorize;
use gh_verify_core::verdict::{RuleResult, Severity};

pub fn print(results: &[RuleResult]) -> Result<()> {
    for r in results {
        match r.severity {
            Severity::Pass => {
                println!(
                    "{} {}: {}",
                    format!("[{}]", r.rule_id).bold(),
                    "pass".green(),
                    r.message
                );
            }
            Severity::Warning => {
                println!(
                    "{} {}: {}",
                    format!("[{}]", r.rule_id).bold(),
                    "warning".yellow(),
                    r.message
                );
                if let Some(ref s) = r.suggestion {
                    print!("{s}");
                }
                println!("  Suggestion: Consider splitting into separate PRs by domain.");
            }
            Severity::Error => {
                println!(
                    "{} {}: {}",
                    format!("[{}]", r.rule_id).bold(),
                    "error".red(),
                    r.message
                );
                if let Some(ref s) = r.suggestion {
                    print!("{s}");
                }
                println!("  Suggestion: Consider splitting into separate PRs by domain.");
            }
        }
    }
    Ok(())
}

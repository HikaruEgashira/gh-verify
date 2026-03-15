pub mod assessment_human;
pub mod assessment_json;
pub mod human;
pub mod json;

use anyhow::Result;
use gh_verify_core::assessment::AssessmentReport;
use gh_verify_core::verdict::RuleResult;

#[derive(Debug, Clone, Copy)]
pub enum Format {
    Human,
    Json,
}

pub fn parse_format(s: &str) -> Result<Format> {
    match s {
        "human" => Ok(Format::Human),
        "json" => Ok(Format::Json),
        _ => anyhow::bail!("invalid format: {s} (use 'human' or 'json')"),
    }
}

pub fn print(format: Format, results: &[RuleResult]) -> Result<()> {
    match format {
        Format::Human => human::print(results),
        Format::Json => json::print(results),
    }
}

pub fn print_assessment(format: Format, report: &AssessmentReport) -> Result<()> {
    match format {
        Format::Human => assessment_human::print(report),
        Format::Json => assessment_json::print(report),
    }
}

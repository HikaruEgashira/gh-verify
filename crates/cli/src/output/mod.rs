pub mod human;
pub mod json;
pub mod sarif;

use anyhow::Result;
use gh_verify_core::assessment::AssessmentReport;

use crate::verify::BatchReport;

#[derive(Debug, Clone, Copy)]
pub enum Format {
    Human,
    Json,
    Sarif,
}

pub fn parse_format(s: &str) -> Result<Format> {
    match s {
        "human" => Ok(Format::Human),
        "json" => Ok(Format::Json),
        "sarif" => Ok(Format::Sarif),
        _ => anyhow::bail!("invalid format: {s} (use 'human', 'json', or 'sarif')"),
    }
}

pub fn print(format: Format, report: &AssessmentReport) -> Result<()> {
    match format {
        Format::Human => human::print(report),
        Format::Json => json::print(report),
        Format::Sarif => sarif::print(report),
    }
}

pub fn print_batch(format: Format, batch: &BatchReport) -> Result<()> {
    match format {
        Format::Human => human::print_batch(batch),
        Format::Json => json::print_batch(batch),
        Format::Sarif => sarif::print_batch(batch),
    }
}

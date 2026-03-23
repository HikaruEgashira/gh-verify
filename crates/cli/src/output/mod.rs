pub mod human;
pub mod json;
pub mod sarif;

use anyhow::Result;
use gh_verify_core::assessment::VerificationResult;

use crate::verify::BatchReport;

#[derive(Debug, Clone, Copy)]
pub enum Format {
    Human,
    Json,
    Sarif,
}

pub struct OutputOptions {
    pub format: Format,
    pub only_failures: bool,
}

pub fn parse_format(s: &str) -> Result<Format> {
    match s {
        "human" => Ok(Format::Human),
        "json" => Ok(Format::Json),
        "sarif" => Ok(Format::Sarif),
        _ => anyhow::bail!("invalid format: {s} (use 'human', 'json', or 'sarif')"),
    }
}

pub fn print(opts: &OutputOptions, result: &VerificationResult) -> Result<()> {
    match opts.format {
        Format::Human => human::print(result, opts.only_failures),
        Format::Json => json::print(result, opts.only_failures),
        Format::Sarif => sarif::print(result, opts.only_failures),
    }
}

pub fn print_batch(opts: &OutputOptions, batch: &BatchReport) -> Result<()> {
    match opts.format {
        Format::Human => human::print_batch(batch, opts.only_failures),
        Format::Json => json::print_batch(batch, opts.only_failures),
        Format::Sarif => sarif::print_batch(batch, opts.only_failures),
    }
}

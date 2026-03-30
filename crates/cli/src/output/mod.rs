pub mod human;

use anyhow::{Context, Result};
use clap::ValueEnum;
use libverify_core::assessment::{BatchReport, VerificationResult};

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Format {
    #[value(name = "human")]
    Human,
    #[value(name = "json")]
    Json,
    #[value(name = "sarif")]
    Sarif,
}

impl std::fmt::Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Format::Human => write!(f, "human"),
            Format::Json => write!(f, "json"),
            Format::Sarif => write!(f, "sarif"),
        }
    }
}

pub struct OutputOptions {
    pub format: Format,
    pub only_failures: bool,
    pub policy: Vec<String>,
    pub excluded: Vec<String>,
    pub output_file: Option<String>,
}

impl OutputOptions {
    /// Single policy name for display, or "default".
    pub fn single_policy(&self) -> &str {
        match self.policy.as_slice() {
            [] => "default",
            [one] => one.as_str(),
            [first, ..] => first.as_str(),
        }
    }
}

fn lib_output_opts(
    format: libverify_output::Format,
    only_failures: bool,
) -> libverify_output::OutputOptions {
    libverify_output::OutputOptions {
        format,
        only_failures,
        tool_name: "gh-verify".to_string(),
        tool_version: env!("GH_VERIFY_VERSION").to_string(),
    }
}

fn emit(output_file: Option<&str>, content: &str) -> Result<()> {
    match output_file {
        Some(path) => {
            std::fs::write(path, format!("{content}\n"))
                .with_context(|| format!("failed to write output to '{path}'"))?;
            Ok(())
        }
        None => {
            println!("{content}");
            Ok(())
        }
    }
}

pub fn print(opts: &OutputOptions, result: &VerificationResult) -> Result<()> {
    if opts.output_file.is_some() && matches!(opts.format, Format::Human) {
        anyhow::bail!("--output-file is only supported with --format json or --format sarif");
    }
    match opts.format {
        Format::Human => human::print(
            result,
            opts.only_failures,
            Some(opts.single_policy()),
            &opts.excluded,
        ),
        Format::Json => {
            let out_opts = lib_output_opts(libverify_output::Format::Json, opts.only_failures);
            let rendered = libverify_output::render(&out_opts, result)?;
            emit(opts.output_file.as_deref(), &rendered)
        }
        Format::Sarif => {
            let out_opts = lib_output_opts(libverify_output::Format::Sarif, opts.only_failures);
            let rendered = libverify_output::render(&out_opts, result)?;
            emit(opts.output_file.as_deref(), &rendered)
        }
    }
}

pub fn print_fleet_matrix(opts: &OutputOptions, matrix: &crate::FleetMatrix) -> Result<()> {
    match opts.format {
        Format::Human => human::print_fleet_matrix(matrix),
        Format::Json => {
            let json = serde_json::to_string_pretty(matrix)
                .context("failed to serialize fleet matrix")?;
            emit(opts.output_file.as_deref(), &json)
        }
        Format::Sarif => {
            anyhow::bail!("SARIF output is not supported for fleet matrix. Use --format json")
        }
    }
}

pub fn print_batch(opts: &OutputOptions, batch: &BatchReport) -> Result<()> {
    if opts.output_file.is_some() && matches!(opts.format, Format::Human) {
        anyhow::bail!("--output-file is only supported with --format json or --format sarif");
    }
    match opts.format {
        Format::Human => human::print_batch(
            batch,
            opts.only_failures,
            Some(opts.single_policy()),
            &opts.excluded,
        ),
        Format::Json => {
            let out_opts = lib_output_opts(libverify_output::Format::Json, opts.only_failures);
            let rendered = libverify_output::render_batch(&out_opts, batch)?;
            emit(opts.output_file.as_deref(), &rendered)
        }
        Format::Sarif => {
            let out_opts = lib_output_opts(libverify_output::Format::Sarif, opts.only_failures);
            let rendered = libverify_output::render_batch(&out_opts, batch)?;
            emit(opts.output_file.as_deref(), &rendered)
        }
    }
}

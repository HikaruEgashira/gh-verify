pub mod human;

use anyhow::Result;
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
    pub policy: Option<String>,
    pub excluded: Vec<String>,
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

pub fn print(opts: &OutputOptions, result: &VerificationResult) -> Result<()> {
    match opts.format {
        Format::Human => human::print(
            result,
            opts.only_failures,
            opts.policy.as_deref(),
            &opts.excluded,
        ),
        Format::Json => {
            let out_opts = lib_output_opts(libverify_output::Format::Json, opts.only_failures);
            println!("{}", libverify_output::render(&out_opts, result)?);
            Ok(())
        }
        Format::Sarif => {
            let out_opts = lib_output_opts(libverify_output::Format::Sarif, opts.only_failures);
            println!("{}", libverify_output::render(&out_opts, result)?);
            Ok(())
        }
    }
}

pub fn print_batch(opts: &OutputOptions, batch: &BatchReport) -> Result<()> {
    match opts.format {
        Format::Human => human::print_batch(
            batch,
            opts.only_failures,
            opts.policy.as_deref(),
            &opts.excluded,
        ),
        Format::Json => {
            let out_opts = lib_output_opts(libverify_output::Format::Json, opts.only_failures);
            println!("{}", libverify_output::render_batch(&out_opts, batch)?);
            Ok(())
        }
        Format::Sarif => {
            let out_opts = lib_output_opts(libverify_output::Format::Sarif, opts.only_failures);
            println!("{}", libverify_output::render_batch(&out_opts, batch)?);
            Ok(())
        }
    }
}

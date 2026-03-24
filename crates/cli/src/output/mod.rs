pub mod human;

use anyhow::Result;
use libverify_core::assessment::{BatchReport, VerificationResult};

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

fn lib_output_opts(format: libverify_output::Format, only_failures: bool) -> libverify_output::OutputOptions {
    libverify_output::OutputOptions {
        format,
        only_failures,
        tool_name: "gh-verify".to_string(),
        tool_version: env!("GH_VERIFY_VERSION").to_string(),
    }
}

pub fn print(opts: &OutputOptions, result: &VerificationResult) -> Result<()> {
    match opts.format {
        Format::Human => human::print(result, opts.only_failures),
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
        Format::Human => human::print_batch(batch, opts.only_failures),
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

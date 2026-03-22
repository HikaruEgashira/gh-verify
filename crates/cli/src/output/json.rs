use anyhow::Result;
use gh_verify_core::assessment::AssessmentReport;

use crate::verify::BatchReport;

pub fn print(report: &AssessmentReport) -> Result<()> {
    let json = serde_json::to_string_pretty(report)?;
    println!("{json}");
    Ok(())
}

pub fn print_batch(batch: &BatchReport) -> Result<()> {
    let json = serde_json::to_string_pretty(batch)?;
    println!("{json}");
    Ok(())
}

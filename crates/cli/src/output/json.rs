use anyhow::Result;
use gh_verify_core::assessment::VerificationResult;

use crate::verify::BatchReport;

pub fn print(result: &VerificationResult) -> Result<()> {
    let json = serde_json::to_string_pretty(result)?;
    println!("{json}");
    Ok(())
}

pub fn print_batch(batch: &BatchReport) -> Result<()> {
    let json = serde_json::to_string_pretty(batch)?;
    println!("{json}");
    Ok(())
}

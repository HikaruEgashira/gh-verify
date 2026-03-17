use anyhow::Result;
use gh_verify_core::assessment::AssessmentReport;

pub fn print(report: &AssessmentReport) -> Result<()> {
    let json = serde_json::to_string_pretty(report)?;
    println!("{json}");
    Ok(())
}

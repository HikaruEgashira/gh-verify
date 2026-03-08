use anyhow::Result;
use gh_verify_core::verdict::RuleResult;

pub fn print(results: &[RuleResult]) -> Result<()> {
    let json = serde_json::to_string_pretty(results)?;
    println!("{json}");
    Ok(())
}

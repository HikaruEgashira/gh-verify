use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use gh_verify_core::verdict::Severity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BenchCase {
    pub id: String,
    pub description: String,
    pub repo: String,
    pub pr_number: u32,
    pub expected: Severity,
    pub rationale: String,
    pub category: String,
    #[serde(default)]
    pub target_rule: Option<String>,
    #[serde(default)]
    pub domains_expected: Vec<String>,
    pub ecosystem: String,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub source: Option<BenchCaseSource>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BenchCaseSource {
    pub provider: String,
    #[serde(default)]
    pub collection_id: Option<u64>,
    #[serde(default)]
    pub collection_name: Option<String>,
    #[serde(default)]
    pub selection: Option<String>,
    #[serde(default)]
    pub discovered_at: Option<String>,
}

pub fn load_cases(dir: &Path) -> Result<Vec<BenchCase>> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .with_context(|| format!("cannot read benchmark cases dir: {}", dir.display()))?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    let mut cases = Vec::new();
    for entry in entries {
        let path = entry.path();
        let content = std::fs::read_to_string(&path)?;
        let case: BenchCase = serde_json::from_str(&content)
            .with_context(|| format!("invalid case: {}", path.display()))?;
        cases.push(case);
    }
    Ok(cases)
}

pub fn write_pretty_json<T: Serialize>(path: impl Into<PathBuf>, value: &T) -> Result<()> {
    let path = path.into();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("cannot create dir: {}", parent.display()))?;
    }

    let json = serde_json::to_string_pretty(value)?;
    std::fs::write(&path, format!("{json}\n"))
        .with_context(|| format!("cannot write json file: {}", path.display()))?;
    Ok(())
}

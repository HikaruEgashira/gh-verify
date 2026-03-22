use anyhow::Result;
use std::process::Command;

use gh_verify_core::evidence::{ArtifactAttestation, EvidenceGap, EvidenceState};

use crate::github::types::ReleaseAsset;

use super::gh_cli;

/// Download release assets to a temporary directory, verify attestations for each,
/// and return an `EvidenceState` suitable for `EvidenceBundle.artifact_attestations`.
///
/// Assets that lack attestations are recorded as unverified rather than causing
/// an error, so the overall assessment can still proceed.
pub fn collect_release_attestations(
    owner: &str,
    repo: &str,
    tag: &str,
    assets: &[ReleaseAsset],
) -> EvidenceState<Vec<ArtifactAttestation>> {
    if assets.is_empty() {
        return EvidenceState::not_applicable();
    }

    let repo_full = format!("{owner}/{repo}");

    // Check whether `gh` CLI is available before doing any work.
    if !gh_cli_available() {
        return EvidenceState::missing(vec![EvidenceGap::CollectionFailed {
            source: "gh-attestation".to_string(),
            subject: "release-assets".to_string(),
            detail: "`gh` CLI is not available".to_string(),
        }]);
    }

    let tmp_dir = match tempfile::tempdir() {
        Ok(d) => d,
        Err(e) => {
            return EvidenceState::missing(vec![EvidenceGap::CollectionFailed {
                source: "gh-attestation".to_string(),
                subject: "release-assets".to_string(),
                detail: format!("failed to create temporary directory: {e}"),
            }]);
        }
    };

    let mut attestations: Vec<ArtifactAttestation> = Vec::new();
    let mut gaps: Vec<EvidenceGap> = Vec::new();

    for asset in assets {
        let asset_path = tmp_dir.path().join(&asset.name);

        // Download the asset using `gh release download`
        match download_asset(owner, repo, tag, &asset.name, &asset_path) {
            Ok(()) => {}
            Err(e) => {
                gaps.push(EvidenceGap::CollectionFailed {
                    source: "gh-release-download".to_string(),
                    subject: asset.name.clone(),
                    detail: format!("failed to download asset: {e}"),
                });
                attestations.push(ArtifactAttestation {
                    subject: asset.name.clone(),
                    predicate_type: String::new(),
                    signer_workflow: None,
                    source_repo: None,
                    verified: false,
                    verification_detail: Some(format!("download failed: {e}")),
                });
                continue;
            }
        }

        // Verify attestation
        let path_str = asset_path.to_string_lossy().to_string();
        match gh_cli::verify_artifact(&path_str, None, Some(&repo_full)) {
            Ok(results) if !results.is_empty() => {
                attestations.extend(gh_cli::to_artifact_attestations(&asset.name, &results));
            }
            Ok(_) => {
                // Empty results — no attestation found
                attestations.push(ArtifactAttestation {
                    subject: asset.name.clone(),
                    predicate_type: String::new(),
                    signer_workflow: None,
                    source_repo: None,
                    verified: false,
                    verification_detail: Some("no attestation found".to_string()),
                });
            }
            Err(e) => {
                // Verification command failed — asset exists but has no valid attestation
                attestations.push(ArtifactAttestation {
                    subject: asset.name.clone(),
                    predicate_type: String::new(),
                    signer_workflow: None,
                    source_repo: None,
                    verified: false,
                    verification_detail: Some(format!("{e}")),
                });
            }
        }
    }

    if gaps.is_empty() {
        EvidenceState::complete(attestations)
    } else {
        EvidenceState::partial(attestations, gaps)
    }
}

/// Download a single release asset using `gh release download`.
fn download_asset(
    owner: &str,
    repo: &str,
    tag: &str,
    asset_name: &str,
    dest: &std::path::Path,
) -> Result<()> {
    let repo_full = format!("{owner}/{repo}");
    let output = Command::new("gh")
        .args([
            "release",
            "download",
            tag,
            "--repo",
            &repo_full,
            "--pattern",
            asset_name,
            "--dir",
            &dest.parent().unwrap().to_string_lossy(),
            "--clobber",
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("{stderr}");
    }

    if !dest.exists() {
        anyhow::bail!("asset file not found after download");
    }

    Ok(())
}

/// Check whether the `gh` CLI is available on PATH.
fn gh_cli_available() -> bool {
    Command::new("gh")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_assets_returns_not_applicable() {
        let result = collect_release_attestations("owner", "repo", "v1.0.0", &[]);
        assert!(matches!(result, EvidenceState::NotApplicable));
    }
}

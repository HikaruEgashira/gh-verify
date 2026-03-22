use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::collections::HashMap;
use std::process::Command;

/// Raw JSON structure from `gh attestation verify --format json`
#[derive(Debug, Deserialize)]
pub struct GhAttestationOutput {
    pub verification_result: VerificationResult,
}

#[derive(Debug, Deserialize)]
pub struct VerificationResult {
    pub statement: Statement,
    pub signature: Option<SignatureInfo>,
}

#[derive(Debug, Deserialize)]
pub struct Statement {
    #[serde(rename = "predicateType")]
    pub predicate_type: String,
    /// In-toto statement subjects: artifacts with their digests.
    #[serde(default)]
    pub subject: Vec<StatementSubject>,
}

/// An in-toto statement subject entry.
#[derive(Debug, Deserialize)]
pub struct StatementSubject {
    pub name: String,
    /// Map of algorithm → hex digest (e.g. {"sha256": "abcd..."}).
    #[serde(default)]
    pub digest: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct SignatureInfo {
    pub certificate: Option<CertificateInfo>,
}

#[derive(Debug, Deserialize)]
pub struct CertificateInfo {
    #[serde(rename = "sourceRepositoryURI")]
    pub source_repository_uri: Option<String>,
    #[serde(rename = "buildSignerURI")]
    pub build_signer_uri: Option<String>,
}

/// Verify an artifact using `gh attestation verify` and return parsed results.
pub fn verify_artifact(
    artifact: &str,
    owner: Option<&str>,
    repo: Option<&str>,
) -> Result<Vec<GhAttestationOutput>> {
    let mut cmd = Command::new("gh");
    cmd.args(["attestation", "verify", artifact, "--format", "json"]);

    if let Some(r) = repo {
        cmd.args(["--repo", r]);
    } else if let Some(o) = owner {
        cmd.args(["--owner", o]);
    } else {
        bail!("either --owner or --repo is required for attestation verification");
    }

    let output = cmd
        .output()
        .context("failed to execute `gh attestation verify`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("gh attestation verify failed: {stderr}");
    }

    let stdout = String::from_utf8(output.stdout).context("invalid UTF-8 in gh output")?;
    // gh outputs a JSON array
    let results: Vec<GhAttestationOutput> =
        serde_json::from_str(&stdout).context("failed to parse gh attestation verify output")?;

    Ok(results)
}

/// Convert parsed `gh attestation verify` results into core evidence types.
///
/// When both a local digest and an attestation-claimed digest are available,
/// the two are compared. A mismatch overrides the `Verified` outcome with
/// `SignatureInvalid` (the attestation does not cover the actual artifact).
pub fn to_artifact_attestations(
    artifact: &str,
    results: &[GhAttestationOutput],
    subject_digest: Option<String>,
) -> Vec<gh_verify_core::evidence::ArtifactAttestation> {
    results
        .iter()
        .map(|r| {
            let cert = r
                .verification_result
                .signature
                .as_ref()
                .and_then(|s| s.certificate.as_ref());

            // Extract the attestation-claimed SHA256 from the in-toto statement subjects.
            let claimed_digest = r
                .verification_result
                .statement
                .subject
                .iter()
                .find_map(|s| s.digest.get("sha256"))
                .map(|hex| format!("sha256:{hex}"));

            // Cross-check local digest against attestation-claimed digest.
            let verification = match (&subject_digest, &claimed_digest) {
                (Some(local), Some(claimed)) if local != claimed => {
                    gh_verify_core::evidence::VerificationOutcome::SignatureInvalid {
                        detail: format!("digest mismatch: local={local}, attestation={claimed}"),
                    }
                }
                _ => gh_verify_core::evidence::VerificationOutcome::Verified,
            };

            gh_verify_core::evidence::ArtifactAttestation {
                subject: artifact.to_string(),
                subject_digest: subject_digest.clone(),
                predicate_type: r.verification_result.statement.predicate_type.clone(),
                signer_workflow: cert.and_then(|c| c.build_signer_uri.clone()),
                source_repo: cert.and_then(|c| c.source_repository_uri.clone()),
                verification,
            }
        })
        .collect()
}

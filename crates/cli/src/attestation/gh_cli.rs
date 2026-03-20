use anyhow::{Context, Result, bail};
use serde::Deserialize;
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

/// Verify an artifact using `gh attestation verify` with full option support.
///
/// Supports binary files, OCI images (`oci://`), custom predicate types,
/// signer workflow pinning, and self-hosted runner rejection.
pub fn verify_artifact_extended(
    artifact: &str,
    owner: Option<&str>,
    repo: Option<&str>,
    digest_alg: &str,
    predicate_type: &str,
    signer_workflow: Option<&str>,
    deny_self_hosted_runners: bool,
) -> Result<Vec<GhAttestationOutput>> {
    let mut cmd = Command::new("gh");
    cmd.args(["attestation", "verify", artifact, "--format", "json"]);
    cmd.args(["--digest-alg", digest_alg]);
    cmd.args(["--predicate-type", predicate_type]);

    if let Some(r) = repo {
        cmd.args(["--repo", r]);
    } else if let Some(o) = owner {
        cmd.args(["--owner", o]);
    } else {
        bail!("either --owner or --repo is required for attestation verification");
    }

    if let Some(workflow) = signer_workflow {
        cmd.args(["--signer-workflow", workflow]);
    }

    if deny_self_hosted_runners {
        cmd.arg("--deny-self-hosted-runners");
    }

    let output = cmd
        .output()
        .context("failed to execute `gh attestation verify`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("gh attestation verify failed: {stderr}");
    }

    let stdout = String::from_utf8(output.stdout).context("invalid UTF-8 in gh output")?;
    let results: Vec<GhAttestationOutput> =
        serde_json::from_str(&stdout).context("failed to parse gh attestation verify output")?;

    Ok(results)
}

/// Convert parsed `gh attestation verify` results into core evidence types.
pub fn to_artifact_attestations(
    artifact: &str,
    results: &[GhAttestationOutput],
) -> Vec<gh_verify_core::evidence::ArtifactAttestation> {
    results
        .iter()
        .map(|r| {
            let cert = r
                .verification_result
                .signature
                .as_ref()
                .and_then(|s| s.certificate.as_ref());

            gh_verify_core::evidence::ArtifactAttestation {
                subject: artifact.to_string(),
                predicate_type: r.verification_result.statement.predicate_type.clone(),
                signer_workflow: cert.and_then(|c| c.build_signer_uri.clone()),
                source_repo: cert.and_then(|c| c.source_repository_uri.clone()),
                verified: true, // gh attestation verify succeeding means verified
                verification_detail: None,
            }
        })
        .collect()
}

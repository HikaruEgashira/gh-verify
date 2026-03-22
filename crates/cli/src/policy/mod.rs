use anyhow::{Context, Result, bail};

use gh_verify_core::control::ControlFinding;
use gh_verify_core::profile::{ControlProfile, FindingSeverity, GateDecision, ProfileOutcome};

const DEFAULT_POLICY: &str = include_str!("default.rego");
const OSS_POLICY: &str = include_str!("oss.rego");
const AIOPS_POLICY: &str = include_str!("aiops.rego");
const RULE_PATH: &str = "data.verify.profile.map";

/// OPA-based profile that evaluates Rego policies to map control findings
/// to gate decisions, enabling per-organization customization.
pub struct OpaProfile {
    engine: regorus::Engine,
    profile_name: &'static str,
}

impl OpaProfile {
    /// Loads a custom Rego policy from the given file path.
    pub fn from_file(path: &str) -> Result<Self> {
        let policy = std::fs::read_to_string(path).with_context(|| {
            format!(
                "reading policy '{path}'. Use a built-in preset (default, oss, aiops) or a path to a .rego file"
            )
        })?;
        Self::from_rego_with_name(path, &policy, "opa-custom")
    }

    /// Creates a profile using the built-in default policy (SLSA Foundation equivalent).
    pub fn default_policy() -> Result<Self> {
        Self::from_rego_with_name("default.rego", DEFAULT_POLICY, "slsa-foundation")
    }

    /// Creates a profile using the built-in OSS preset.
    /// Tolerates unsigned commits and self-reviewed merges.
    pub fn oss_preset() -> Result<Self> {
        Self::from_rego_with_name("oss.rego", OSS_POLICY, "oss")
    }

    /// Creates a profile using the built-in AI-ops audit preset.
    /// Maps all indeterminate findings to review instead of fail.
    pub fn aiops_preset() -> Result<Self> {
        Self::from_rego_with_name("aiops.rego", AIOPS_POLICY, "aiops")
    }

    /// Loads a built-in preset by name, or falls back to file path.
    /// Recognised preset names: "default", "oss", "aiops".
    pub fn from_preset_or_file(name: &str) -> Result<Self> {
        match name {
            "default" => Self::default_policy(),
            "oss" => Self::oss_preset(),
            "aiops" => Self::aiops_preset(),
            path => Self::from_file(path),
        }
    }

    fn from_rego_with_name(name: &str, rego: &str, profile_name: &'static str) -> Result<Self> {
        let mut engine = regorus::Engine::new();
        engine
            .add_policy(name.to_string(), rego.to_string())
            .with_context(|| format!("parsing policy {name}"))?;
        Ok(Self { engine, profile_name })
    }

    fn eval_finding(&self, finding: &ControlFinding) -> Result<(FindingSeverity, GateDecision)> {
        let input_json = serde_json::to_string(finding).context("serializing finding to JSON")?;

        // Engine requires &mut self for eval, so clone per-evaluation.
        // Findings are few (one per control x subject), so this is acceptable.
        let mut engine = self.engine.clone();
        engine.set_input(regorus::Value::from_json_str(&input_json).context("parsing input")?);

        let result = engine
            .eval_rule(RULE_PATH.to_string())
            .context("evaluating OPA rule")?;

        let severity = result["severity"]
            .as_string()
            .context("policy output missing 'severity' string field")?;
        let decision = result["decision"]
            .as_string()
            .context("policy output missing 'decision' string field")?;

        let severity = parse_severity(severity.as_ref())?;
        let decision = parse_decision(decision.as_ref())?;
        Ok((severity, decision))
    }
}

impl ControlProfile for OpaProfile {
    fn name(&self) -> &'static str {
        self.profile_name
    }

    fn map(&self, finding: &ControlFinding) -> ProfileOutcome {
        let (severity, decision) = self.eval_finding(finding).unwrap_or_else(|err| {
            eprintln!(
                "Warning: OPA evaluation failed for {}: {err:#}. Defaulting to Fail.",
                finding.control_id
            );
            (FindingSeverity::Error, GateDecision::Fail)
        });

        ProfileOutcome {
            control_id: finding.control_id,
            severity,
            decision,
            rationale: finding.rationale.clone(),
        }
    }
}

fn parse_severity(s: &str) -> Result<FindingSeverity> {
    match s {
        "info" => Ok(FindingSeverity::Info),
        "warning" => Ok(FindingSeverity::Warning),
        "error" => Ok(FindingSeverity::Error),
        _ => bail!("invalid severity '{s}': expected info, warning, or error"),
    }
}

fn parse_decision(s: &str) -> Result<GateDecision> {
    match s {
        "pass" => Ok(GateDecision::Pass),
        "review" => Ok(GateDecision::Review),
        "fail" => Ok(GateDecision::Fail),
        _ => bail!("invalid decision '{s}': expected pass, review, or fail"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gh_verify_core::control::{ControlId, ControlStatus};

    fn make_finding(control_id: ControlId, status: ControlStatus) -> ControlFinding {
        match status {
            ControlStatus::Satisfied => {
                ControlFinding::satisfied(control_id, "test rationale", vec!["subject".into()])
            }
            ControlStatus::Violated => {
                ControlFinding::violated(control_id, "test rationale", vec!["subject".into()])
            }
            ControlStatus::Indeterminate => ControlFinding::indeterminate(
                control_id,
                "test rationale",
                vec!["subject".into()],
                vec![],
            ),
            ControlStatus::NotApplicable => {
                ControlFinding::not_applicable(control_id, "test rationale")
            }
        }
    }

    #[test]
    fn default_policy_matches_slsa_foundation() {
        use gh_verify_core::profile::SlsaFoundationProfile;

        let opa = OpaProfile::default_policy().unwrap();
        let slsa = SlsaFoundationProfile;

        let cases = [
            (ControlId::ReviewIndependence, ControlStatus::Satisfied),
            (ControlId::ReviewIndependence, ControlStatus::Violated),
            (ControlId::ReviewIndependence, ControlStatus::Indeterminate),
            (ControlId::SourceAuthenticity, ControlStatus::NotApplicable),
            (ControlId::SourceAuthenticity, ControlStatus::Violated),
        ];

        for (id, status) in &cases {
            let finding = make_finding(*id, *status);
            let opa_outcome = opa.map(&finding);
            let slsa_outcome = slsa.map(&finding);

            assert_eq!(
                opa_outcome.severity, slsa_outcome.severity,
                "severity mismatch for {id:?}/{status:?}"
            );
            assert_eq!(
                opa_outcome.decision, slsa_outcome.decision,
                "decision mismatch for {id:?}/{status:?}"
            );
        }
    }

    #[test]
    fn custom_policy_indeterminate_to_review() {
        let custom_rego = r#"
package verify.profile

import rego.v1

default map := {"severity": "error", "decision": "fail"}

map := {"severity": "info", "decision": "pass"} if {
    input.status == "satisfied"
}

map := {"severity": "info", "decision": "pass"} if {
    input.status == "not_applicable"
}

map := {"severity": "warning", "decision": "review"} if {
    input.status == "indeterminate"
}

map := {"severity": "error", "decision": "fail"} if {
    input.status == "violated"
}
"#;
        let profile = OpaProfile::from_rego_with_name("custom.rego", custom_rego, "opa-custom").unwrap();

        let finding = make_finding(ControlId::ReviewIndependence, ControlStatus::Indeterminate);
        let outcome = profile.map(&finding);

        assert_eq!(outcome.severity, FindingSeverity::Warning);
        assert_eq!(outcome.decision, GateDecision::Review);
    }

    #[test]
    fn custom_policy_per_control_override() {
        let custom_rego = r#"
package verify.profile

import rego.v1

default map := {"severity": "error", "decision": "fail"}

map := {"severity": "info", "decision": "pass"} if {
    input.status == "satisfied"
}

map := {"severity": "info", "decision": "pass"} if {
    input.status == "not_applicable"
}

# source-authenticity violations get review instead of fail
map := {"severity": "warning", "decision": "review"} if {
    input.control_id == "source-authenticity"
    input.status == "violated"
}

map := {"severity": "error", "decision": "fail"} if {
    input.status == "indeterminate"
}
"#;
        let profile = OpaProfile::from_rego_with_name("custom.rego", custom_rego, "opa-custom").unwrap();

        // source-authenticity violated -> review
        let finding = make_finding(ControlId::SourceAuthenticity, ControlStatus::Violated);
        let outcome = profile.map(&finding);
        assert_eq!(outcome.decision, GateDecision::Review);

        // review-independence violated -> still fail (default)
        let finding = make_finding(ControlId::ReviewIndependence, ControlStatus::Violated);
        let outcome = profile.map(&finding);
        assert_eq!(outcome.decision, GateDecision::Fail);
    }

    #[test]
    fn invalid_policy_returns_error() {
        let result = OpaProfile::from_rego_with_name("bad.rego", "this is not valid rego!!!", "opa-custom");
        assert!(result.is_err());
    }

    // --- OSS preset tests ---

    #[test]
    fn oss_preset_source_authenticity_violated_is_review() {
        let profile = OpaProfile::oss_preset().unwrap();
        let finding = make_finding(ControlId::SourceAuthenticity, ControlStatus::Violated);
        let outcome = profile.map(&finding);
        assert_eq!(outcome.severity, FindingSeverity::Warning);
        assert_eq!(outcome.decision, GateDecision::Review);
    }

    #[test]
    fn oss_preset_review_independence_indeterminate_is_review() {
        let profile = OpaProfile::oss_preset().unwrap();
        let finding = make_finding(ControlId::ReviewIndependence, ControlStatus::Indeterminate);
        let outcome = profile.map(&finding);
        assert_eq!(outcome.severity, FindingSeverity::Warning);
        assert_eq!(outcome.decision, GateDecision::Review);
    }

    #[test]
    fn oss_preset_other_violated_still_fails() {
        let profile = OpaProfile::oss_preset().unwrap();
        let finding = make_finding(ControlId::ReviewIndependence, ControlStatus::Violated);
        let outcome = profile.map(&finding);
        assert_eq!(outcome.severity, FindingSeverity::Error);
        assert_eq!(outcome.decision, GateDecision::Fail);
    }

    #[test]
    fn oss_preset_satisfied_passes() {
        let profile = OpaProfile::oss_preset().unwrap();
        let finding = make_finding(ControlId::SourceAuthenticity, ControlStatus::Satisfied);
        let outcome = profile.map(&finding);
        assert_eq!(outcome.severity, FindingSeverity::Info);
        assert_eq!(outcome.decision, GateDecision::Pass);
    }

    // --- AI-ops preset tests ---

    #[test]
    fn aiops_preset_indeterminate_is_review() {
        let profile = OpaProfile::aiops_preset().unwrap();
        let finding = make_finding(ControlId::ReviewIndependence, ControlStatus::Indeterminate);
        let outcome = profile.map(&finding);
        assert_eq!(outcome.severity, FindingSeverity::Warning);
        assert_eq!(outcome.decision, GateDecision::Review);
    }

    #[test]
    fn aiops_preset_violated_still_fails() {
        let profile = OpaProfile::aiops_preset().unwrap();
        let finding = make_finding(ControlId::SourceAuthenticity, ControlStatus::Violated);
        let outcome = profile.map(&finding);
        assert_eq!(outcome.severity, FindingSeverity::Error);
        assert_eq!(outcome.decision, GateDecision::Fail);
    }

    #[test]
    fn aiops_preset_satisfied_passes() {
        let profile = OpaProfile::aiops_preset().unwrap();
        let finding = make_finding(ControlId::SourceAuthenticity, ControlStatus::Satisfied);
        let outcome = profile.map(&finding);
        assert_eq!(outcome.severity, FindingSeverity::Info);
        assert_eq!(outcome.decision, GateDecision::Pass);
    }

    // --- from_preset_or_file tests ---

    #[test]
    fn from_preset_or_file_resolves_named_presets() {
        assert!(OpaProfile::from_preset_or_file("default").is_ok());
        assert!(OpaProfile::from_preset_or_file("oss").is_ok());
        assert!(OpaProfile::from_preset_or_file("aiops").is_ok());
    }
}


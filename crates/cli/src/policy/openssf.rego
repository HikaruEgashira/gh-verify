# OpenSSF Scorecard aligned policy preset.
#
# Maps gh-verify controls to OpenSSF Scorecard checks.
# Strict on supply-chain integrity checks; advisory on code quality checks.
#
# Scorecard Mapping:
#   Branch-Protection      → branch-protection-enforcement, branch-history-integrity
#   Code-Review            → review-independence, two-party-review, stale-review
#   CI-Tests               → required-status-checks
#   Signed-Releases        → source-authenticity, provenance-authenticity, build-provenance
#   SAST                   → sast-tool-presence
#   Binary-Artifacts       → binary-artifact-check
#   Pinned-Dependencies    → dependency-pinning
#   Token-Permissions      → workflow-permissions
#   Dangerous-Workflow     → security-file-change (partial)
#
# Non-Scorecard (SOC2 change management) controls are advisory.
#
# Input/Output schema: see default.rego

package verify.profile

import rego.v1

default map := {"severity": "error", "decision": "fail"}

map := {"severity": "info", "decision": "pass"} if {
	input.status == "satisfied"
}

map := {"severity": "info", "decision": "pass"} if {
	input.status == "not_applicable"
}

# --- Supply chain integrity: strict ---
# binary-artifact-check, dependency-pinning, workflow-permissions
# Violated → fail (supply chain attacks are high-impact)

# --- SAST: strict on violated, advisory on indeterminate ---
map := {"severity": "warning", "decision": "review"} if {
	input.control_id == "sast-tool-presence"
	input.status == "indeterminate"
}

# --- Code quality / change management: advisory ---
# These don't directly map to Scorecard checks; treat as informational.
openssf_advisory_controls contains "pr-size"
openssf_advisory_controls contains "test-coverage"
openssf_advisory_controls contains "scoped-change"
openssf_advisory_controls contains "description-quality"
openssf_advisory_controls contains "conventional-title"
openssf_advisory_controls contains "merge-commit-policy"
openssf_advisory_controls contains "issue-linkage"
openssf_advisory_controls contains "release-traceability"

map := {"severity": "warning", "decision": "review"} if {
	input.control_id in openssf_advisory_controls
	input.status == "violated"
}

map := {"severity": "warning", "decision": "review"} if {
	input.control_id in openssf_advisory_controls
	input.status == "indeterminate"
}

# --- security-file-change: advisory (detection, not blocking) ---
map := {"severity": "warning", "decision": "review"} if {
	input.control_id == "security-file-change"
	input.status == "violated"
}

map := {"severity": "warning", "decision": "review"} if {
	input.control_id == "security-file-change"
	input.status == "indeterminate"
}

# --- Generic fallthrough ---
map := {"severity": "error", "decision": "fail"} if {
	input.status == "indeterminate"
	not input.control_id in openssf_advisory_controls
	input.control_id != "sast-tool-presence"
	input.control_id != "security-file-change"
}

map := {"severity": "error", "decision": "fail"} if {
	input.status == "violated"
	not input.control_id in openssf_advisory_controls
	input.control_id != "security-file-change"
}

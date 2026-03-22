# NIST SSDF (SP 800-218) aligned policy preset.
#
# Maps gh-verify controls to NIST SSDF practice areas.
# Strict on PS (Protect Software) and PW (Produce Well-Secured Software)
# controls; advisory on organizational/process controls that cannot be
# fully verified from PR/release evidence alone.
#
# SSDF Mapping:
#   PS.1 (Protect code)     → source-authenticity, branch-*, dependency-pinning,
#                              workflow-permissions, binary-artifact-check
#   PS.2 (Release integrity) → build-provenance, provenance-authenticity, signed-releases
#   PW.5 (Secure build)     → hosted-build-platform, build-isolation
#   PW.6 (Code review)      → review-independence, two-party-review, stale-review
#   PW.7 (Vuln testing)     → sast-tool-presence, required-status-checks, test-coverage
#   CC7/CC8 (Change mgmt)   → pr-size, scoped-change, description-quality,
#                              conventional-title, issue-linkage, release-traceability,
#                              security-file-change, merge-commit-policy
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

# --- PS.1 / PS.2: Protect Software — strict ---
# Violated → fail (code/release integrity is non-negotiable)

# --- PW.7: Vulnerability testing — strict on presence, advisory on specifics ---
# sast-tool-presence indeterminate → review (CI data may be unavailable)
map := {"severity": "warning", "decision": "review"} if {
	input.control_id == "sast-tool-presence"
	input.status == "indeterminate"
}

# test-coverage indeterminate → review (test matching is heuristic)
map := {"severity": "warning", "decision": "review"} if {
	input.control_id == "test-coverage"
	input.status == "indeterminate"
}

# --- CC7/CC8: Change management — advisory ---
# These are process quality controls; violated triggers review, not gate failure.
nist_advisory_controls contains "pr-size"
nist_advisory_controls contains "scoped-change"
nist_advisory_controls contains "description-quality"
nist_advisory_controls contains "conventional-title"
nist_advisory_controls contains "merge-commit-policy"
nist_advisory_controls contains "security-file-change"

map := {"severity": "warning", "decision": "review"} if {
	input.control_id in nist_advisory_controls
	input.status == "violated"
}

map := {"severity": "warning", "decision": "review"} if {
	input.control_id in nist_advisory_controls
	input.status == "indeterminate"
}

# --- Generic fallthrough ---
map := {"severity": "error", "decision": "fail"} if {
	input.status == "indeterminate"
	not input.control_id in nist_advisory_controls
	input.control_id != "sast-tool-presence"
	input.control_id != "test-coverage"
}

map := {"severity": "error", "decision": "fail"} if {
	input.status == "violated"
	not input.control_id in nist_advisory_controls
}

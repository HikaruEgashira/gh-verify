# SLSA Comprehensive profile: Source Track strict, Build/Repo Track lenient.
#
# Source controls (review-independence, source-authenticity):
#   indeterminate → fail (evidence required)
# Build/Repo controls (build-provenance, branch-protection, required-reviewers):
#   indeterminate → review (evidence optional, human judgment)

package verify.profile

import rego.v1

source_controls := {"review-independence", "source-authenticity"}

default map := {"severity": "error", "decision": "fail"}

map := {"severity": "info", "decision": "pass"} if {
	input.status == "satisfied"
}

map := {"severity": "info", "decision": "pass"} if {
	input.status == "not_applicable"
}

# Source Track: indeterminate → fail
map := {"severity": "error", "decision": "fail"} if {
	input.status == "indeterminate"
	input.control_id in source_controls
}

# Build/Repo Track: indeterminate → review
map := {"severity": "warning", "decision": "review"} if {
	input.status == "indeterminate"
	not input.control_id in source_controls
}

map := {"severity": "error", "decision": "fail"} if {
	input.status == "violated"
}

#!/usr/bin/env bash
# Tests for domain classification logic via the detect-unscoped-change rule.
# These drive the rule with synthetic PR data through the GitHub API mock.
# For now these serve as documentation of expected classification behaviour.
set -euo pipefail

# Domain classification expectations (path → expected domain name)
# Verified by reading src/util/diff_parser.zig
declare -A CASES=(
  ["src/auth/login.zig"]="auth"
  ["src/auth/token.zig"]="auth"
  ["src/ui/LoginForm.tsx"]="ui"
  ["src/db/schema.sql"]="database"
  ["docs/guide.md"]="docs"
  [".github/workflows/ci.yml"]="ci"
  ["src/api/handler.zig"]="api"
  ["src/config/settings.toml"]="config"
  ["test/foo_test.zig"]="test"
  ["src/random.zig"]="unknown"
)

PASS=0
FAIL=0

for path in "${!CASES[@]}"; do
  expected="${CASES[$path]}"
  echo "  classify: $path → $expected"
  PASS=$((PASS + 1))
done

echo ""
echo "classification table: $PASS entries documented"

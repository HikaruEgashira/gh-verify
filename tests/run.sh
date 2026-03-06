#!/usr/bin/env bash
# Test runner for gh-lint.
# Usage: bash tests/run.sh [--filter <name>]
set -euo pipefail

BINARY="${BINARY:-./zig-out/bin/gh-lint}"
FILTER="${2:-}"
PASS=0
FAIL=0

run_test() {
  local name="$1"
  local fn="$2"
  if [[ -n "$FILTER" && "$name" != *"$FILTER"* ]]; then
    return
  fi
  if $fn; then
    echo "  ok  $name"
    PASS=$((PASS + 1))
  else
    echo "  FAIL $name"
    FAIL=$((FAIL + 1))
  fi
}

# Build first
zig build 2>/dev/null

# ── Unit-style tests (no network) ─────────────────────────────────────

test_version() {
  local out
  out=$("$BINARY" version 2>&1)
  [[ "$out" == *"gh-lint"* ]]
}

test_help() {
  local out
  out=$("$BINARY" help 2>&1)
  [[ "$out" == *"Usage"* ]]
}

test_unknown_subcmd_exits_1() {
  "$BINARY" notacommand 2>/dev/null
  [[ $? -ne 0 ]]
}

test_pr_list_rules() {
  local out
  # Fake token to bypass network; list-rules does not hit the API
  GH_TOKEN=fake GH_REPO=fake/fake out=$("$BINARY" pr list-rules 2>&1) || true
  [[ "$out" == *"detect-unscoped-change"* ]]
}

echo "ghlint tests"
echo "binary: $BINARY"
echo ""

run_test "version output"           test_version
run_test "help output"              test_help
run_test "unknown subcmd exits 1"   test_unknown_subcmd_exits_1
run_test "pr list-rules"            test_pr_list_rules

echo ""
echo "results: $PASS passed, $FAIL failed"
[[ $FAIL -eq 0 ]]

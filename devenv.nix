{ pkgs, ... }: {
  packages = [
    pkgs.zig_0_15
    pkgs.gh
    pkgs.jq
  ];

  scripts = {
    # zig build -Doptimize=ReleaseSafe
    build.exec = ''
      zig build -Doptimize=ReleaseSafe "$@"
    '';

    # Unit tests (no network required)
    lint-test.exec = ''
      set -euo pipefail
      BINARY="''${BINARY:-./zig-out/bin/gh-lint}"
      FILTER="''${1:-}"
      PASS=0
      FAIL=0

      run_test() {
        local name="$1" fn="$2"
        if [[ -n "$FILTER" && "$name" != *"$FILTER"* ]]; then return; fi
        if $fn; then
          echo "  ok  $name"; PASS=$((PASS + 1))
        else
          echo "  FAIL $name"; FAIL=$((FAIL + 1))
        fi
      }

      zig build 2>/dev/null

      test_version()             { local o; o=$("$BINARY" version 2>&1); [[ "$o" == *"gh-lint"* ]]; }
      test_help()                { local o; o=$("$BINARY" help 2>&1);    [[ "$o" == *"Usage"* ]]; }
      test_unknown_subcmd_exits_1() { "$BINARY" notacommand 2>/dev/null; [[ $? -ne 0 ]]; }
      test_pr_list_rules()       {
        local o
        GH_TOKEN=fake GH_REPO=fake/fake o=$("$BINARY" pr list-rules 2>&1) || true
        [[ "$o" == *"detect-unscoped-change"* ]]
      }

      echo "ghlint tests"
      echo "binary: $BINARY"
      echo ""
      run_test "version output"          test_version
      run_test "help output"             test_help
      run_test "unknown subcmd exits 1"  test_unknown_subcmd_exits_1
      run_test "pr list-rules"           test_pr_list_rules

      echo ""
      echo "results: $PASS passed, $FAIL failed"
      [[ $FAIL -eq 0 ]]
    '';

    # Domain classification documentation table
    lint-classify.exec = ''
      set -euo pipefail
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
      COUNT=0
      for path in "''${!CASES[@]}"; do
        echo "  classify: $path → ''${CASES[$path]}"
        COUNT=$((COUNT + 1))
      done
      echo ""
      echo "classification table: $COUNT entries documented"
    '';

    # Benchmark runner against real PRs
    lint-bench.exec = ''
      set -uo pipefail
      SCRIPT_DIR="$(cd "$(dirname "''${BASH_SOURCE[0]}")" && pwd)"
      CASES_DIR="$SCRIPT_DIR/benchmarks/cases"
      RESULTS_DIR="$SCRIPT_DIR/benchmarks/results"
      GHLINT="''${GHLINT_BIN:-./zig-out/bin/gh-lint}"
      TIMESTAMP="$(date -u +%Y%m%dT%H%M%SZ)"
      OUTPUT_FILE="$RESULTS_DIR/run_''${TIMESTAMP}.json"

      GREEN="\033[32m" RED="\033[31m" YELLOW="\033[33m" RESET="\033[0m"
      mkdir -p "$RESULTS_DIR"
      pass_count=0; fail_count=0; results_json="["; first=true

      run_case() {
        local case_file="$1"
        local id repo pr_number expected category description rationale notes
        id=$(jq -r '.id' "$case_file")
        repo=$(jq -r '.repo' "$case_file")
        pr_number=$(jq -r '.pr_number' "$case_file")
        expected=$(jq -r '.expected' "$case_file")
        category=$(jq -r '.category' "$case_file")
        description=$(jq -r '.description' "$case_file")
        rationale=$(jq -r '.rationale' "$case_file")
        notes=$(jq -r '.notes // ""' "$case_file")

        local actual_json actual_severity
        actual_json=$(GH_TOKEN="''${GH_TOKEN:-$(gh auth token 2>/dev/null || echo "")}" \
          "$GHLINT" pr "$pr_number" --repo "$repo" --format json 2>/dev/null; true)
        if ! echo "$actual_json" | jq empty 2>/dev/null; then actual_json="[]"; fi
        actual_severity=$(echo "$actual_json" | jq -r '.[0].severity // "fetch_error"')

        local outcome
        if [[ "$actual_severity" == "$expected" ]]; then
          outcome="PASS"; pass_count=$((pass_count + 1))
          printf "''${GREEN}[PASS]''${RESET} %s | %s#%s | expected=%s\n" "$id" "$repo" "$pr_number" "$expected"
        else
          outcome="FAIL"; fail_count=$((fail_count + 1))
          printf "''${RED}[FAIL]''${RESET} %s | %s#%s | expected=%s actual=%s\n" "$id" "$repo" "$pr_number" "$expected" "$actual_severity"
          [[ -n "$notes" ]] && printf "       ''${YELLOW}note: %s''${RESET}\n" "$notes"
        fi

        local entry
        entry=$(jq -n \
          --arg id "$id" --arg repo "$repo" --argjson pr "$pr_number" \
          --arg expected "$expected" --arg actual "$actual_severity" \
          --arg outcome "$outcome" --arg category "$category" \
          --arg description "$description" --arg rationale "$rationale" \
          --arg notes "$notes" --argjson raw_output "$actual_json" \
          '{id:$id,repo:$repo,pr_number:$pr,expected:$expected,actual:$actual,
            outcome:$outcome,category:$category,description:$description,
            rationale:$rationale,notes:$notes,raw_output:$raw_output}')
        if [[ "$first" == "true" ]]; then results_json+="$entry"; first=false
        else results_json+=",$entry"; fi
      }

      echo ""; echo "ghlint benchmark"; echo "================"; echo "binary: $GHLINT"; echo ""
      for case_file in "$CASES_DIR"/pass/*.json "$CASES_DIR"/warn/*.json "$CASES_DIR"/error/*.json; do
        [[ -f "$case_file" ]] && run_case "$case_file"
      done
      results_json+="]"
      total=$((pass_count + fail_count))
      echo ""; echo "========================="
      if [[ "$fail_count" -eq 0 ]]; then
        printf "''${GREEN}All %d cases passed''${RESET}\n" "$total"
      else
        printf "''${RED}%d/%d cases failed''${RESET}\n" "$fail_count" "$total"
      fi

      jq -n \
        --arg timestamp "$TIMESTAMP" --arg ghlint_bin "$GHLINT" \
        --argjson pass_count "$pass_count" --argjson fail_count "$fail_count" \
        --argjson total "$total" --argjson results "$results_json" \
        '{timestamp:$timestamp,ghlint_bin:$ghlint_bin,
          summary:{total:$total,pass:$pass_count,fail:$fail_count,
            pass_rate_pct:(if $total>0 then ($pass_count*100/$total|floor) else 0 end)},
          results:$results}' > "$OUTPUT_FILE"
      echo "Report: $OUTPUT_FILE"
      exit "$fail_count"
    '';

    # Cross-compile release binaries (CI use; zig provided by devenv)
    dist-build.exec = ''
      set -euo pipefail
      TAG="''${1:-dev}"
      EXT_NAME="gh-lint"
      mkdir -p dist

      declare -A TARGETS=(
        ["aarch64-macos"]="darwin arm64"
        ["x86_64-macos"]="darwin amd64"
        ["x86_64-linux-musl"]="linux amd64"
        ["aarch64-linux-musl"]="linux arm64"
        ["x86_64-windows"]="windows amd64"
      )
      for ZIG_TARGET in "''${!TARGETS[@]}"; do
        read -r OS ARCH <<< "''${TARGETS[$ZIG_TARGET]}"
        EXT=""; [[ "$OS" == "windows" ]] && EXT=".exe"
        echo "Building for ''${ZIG_TARGET}..."
        zig build -Dtarget="''${ZIG_TARGET}" -Doptimize=ReleaseSafe
        OUTPUT_NAME="''${EXT_NAME}_''${TAG}_''${OS}-''${ARCH}''${EXT}"
        cp "zig-out/bin/''${EXT_NAME}''${EXT}" "dist/''${OUTPUT_NAME}"
        echo "  -> dist/''${OUTPUT_NAME}"
      done
      echo "Build complete."
    '';
  };

  enterShell = ''
    echo "ghlint dev environment"
    echo "  zig : $(zig version)"
    echo "  gh  : $(gh --version | head -1)"
  '';
}

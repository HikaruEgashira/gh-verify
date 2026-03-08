{ pkgs, ... }: {
  cachix.enable = false;

  packages = [
    pkgs.zig_0_15
    pkgs.gh
    pkgs.jq
    pkgs.python3
  ];

  tasks = {
    "ghverify:build" = {
      description = "Build release binary";
      exec = "zig build -Doptimize=ReleaseSafe";
    };

    "ghverify:test" = {
      description = "Unit tests (no network required)";
      after = [ "ghverify:build" ];
      exec = ''
        set -euo pipefail
        BINARY="$DEVENV_ROOT/zig-out/bin/gh-verify"
        PASS=0; FAIL=0

        run_test() {
          local name="$1" fn="$2"
          if $fn; then echo "  ok  $name"; PASS=$((PASS+1))
          else         echo "  FAIL $name"; FAIL=$((FAIL+1)); fi
        }

        test_version()  { local o; o=$("$BINARY" version 2>&1); [[ "$o" == *"gh-verify"* ]]; }
        test_help()     { local o; o=$("$BINARY" help 2>&1);    [[ "$o" == *"Usage"* ]]; }
        test_exits_1()  { "$BINARY" notacommand 2>/dev/null; [[ $? -ne 0 ]]; }
        test_rules()    { local o; GH_TOKEN=fake GH_REPO=fake/fake o=$("$BINARY" pr list-rules 2>&1) || true; [[ "$o" == *"detect-unscoped-change"* ]]; }

        echo "ghverify tests"; echo ""
        run_test "version output"         test_version
        run_test "help output"            test_help
        run_test "unknown subcmd exits 1" test_exits_1
        run_test "pr list-rules"          test_rules
        echo ""; echo "results: $PASS passed, $FAIL failed"
        [[ $FAIL -eq 0 ]]
      '';
    };

    "ghverify:bench:run" = {
      description = "Run ghverify against all benchmark cases and collect raw results";
      after = [ "ghverify:build" ];
      exec = ''
        set -euo pipefail
        GHVERIFY="$DEVENV_ROOT/zig-out/bin/gh-verify"
        TIMESTAMP="$(date -u +%Y%m%dT%H%M%SZ)"
        RAW_FILE="$DEVENV_ROOT/benchmarks/results/raw_''${TIMESTAMP}.jsonl"
        echo ""; echo "ghverify benchmark"; echo "================"; echo ""
        python3 "$DEVENV_ROOT/benchmarks/collect.py" "$GHVERIFY" "$RAW_FILE"
      '';
    };

    "ghverify:bench:report" = {
      description = "Compute Accuracy, Precision, Recall, F1 from raw benchmark results";
      exec = ''
        set -euo pipefail
        RESULTS_DIR="$DEVENV_ROOT/benchmarks/results"
        RAW_FILE=$(ls -t "$RESULTS_DIR"/raw_*.jsonl 2>/dev/null | head -1)
        if [[ -z "$RAW_FILE" ]]; then echo "No raw results found. Run ghverify:bench:run first."; exit 1; fi
        TIMESTAMP=$(basename "$RAW_FILE" .jsonl | sed 's/raw_//')
        OUTPUT_FILE="$RESULTS_DIR/run_''${TIMESTAMP}.json"

        python3 "$DEVENV_ROOT/benchmarks/report.py" "$RAW_FILE" "$OUTPUT_FILE"
      '';
    };

    "ghverify:bench" = {
      description = "Run benchmarks and generate report";
      after = [ "ghverify:bench:run" "ghverify:bench:report" ];
      exec = "echo 'Benchmark complete.'";
    };

    "ghverify:dist" = {
      description = "Cross-compile release binaries for all platforms";
      exec = ''
        set -euo pipefail
        TAG="''${1:-dev}"
        EXT_NAME="gh-verify"
        mkdir -p dist

        declare -A TARGETS=(
          ["aarch64-macos"]="darwin arm64"   ["x86_64-macos"]="darwin amd64"
          ["x86_64-linux-musl"]="linux amd64" ["aarch64-linux-musl"]="linux arm64"
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
  };

  enterShell = ''
    echo "ghverify dev environment"
    echo "  zig : $(zig version)"
    echo "  gh  : $(gh --version | head -1)"
  '';
}

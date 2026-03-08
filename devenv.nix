{ pkgs, ... }: {
  cachix.enable = false;

  packages = [
    pkgs.gh
    pkgs.jq
    pkgs.python3
  ];

  languages.rust = {
    enable = true;
    channel = "stable";
  };

  tasks = {
    "ghverify:build" = {
      description = "Build release binary";
      exec = "cargo build --release -p gh-verify";
    };

    "ghverify:test" = {
      description = "Unit tests (no network required)";
      exec = ''
        set -euo pipefail
        echo "ghverify tests"; echo ""

        # Run all Rust unit tests
        cargo test --workspace 2>&1

        # Integration tests with binary
        BINARY="$DEVENV_ROOT/target/release/gh-verify"
        if [ ! -f "$BINARY" ]; then
          cargo build --release -p gh-verify
        fi
        PASS=0; FAIL=0

        run_test() {
          local name="$1" fn="$2"
          if $fn; then echo "  ok  $name"; PASS=$((PASS+1))
          else         echo "  FAIL $name"; FAIL=$((FAIL+1)); fi
        }

        test_version()  { local o; o=$("$BINARY" --version 2>&1); [[ "$o" == *"gh-verify"* ]]; }
        test_help()     { local o; o=$("$BINARY" --help 2>&1);    [[ "$o" == *"Usage"* ]]; }
        test_exits_1()  { "$BINARY" notacommand 2>/dev/null; [[ $? -ne 0 ]]; }
        test_rules()    { local o; GH_TOKEN=fake GH_REPO=fake/fake o=$("$BINARY" pr list-rules 2>&1) || true; [[ "$o" == *"detect-unscoped-change"* ]]; }

        echo ""; echo "Integration tests:"
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
        GHVERIFY="$DEVENV_ROOT/target/release/gh-verify"
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
      description = "Build release binaries using cargo";
      exec = ''
        set -euo pipefail
        TAG="''${1:-dev}"
        EXT_NAME="gh-verify"
        mkdir -p dist

        echo "Building release binary..."
        cargo build --release -p gh-verify
        cp "target/release/''${EXT_NAME}" "dist/''${EXT_NAME}_''${TAG}_$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m)"
        echo "Build complete."
      '';
    };

    "ghverify:fmt" = {
      description = "Format and lint all Rust code";
      exec = ''
        set -euo pipefail
        cargo fmt --all
        cargo clippy --workspace -- -D warnings
      '';
    };

    "ghverify:verify" = {
      description = "Run Creusot formal verification on verif crate";
      exec = ''
        set -euo pipefail
        eval $(opam env --switch=creusot 2>/dev/null) || { echo "Creusot not installed. See HACKING.md for setup."; exit 1; }
        cargo creusot -p gh-verify-verif
        echo "Translation complete. Use 'cargo creusot prove \"<fn>\" -- -p gh-verify-verif' to prove individual functions."
      '';
    };
  };

  enterShell = ''
    echo "ghverify dev environment"
    echo "  rust: $(rustc --version)"
    echo "  gh  : $(gh --version | head -1)"
  '';
}

{ pkgs, ... }: {
  cachix.enable = false;

  packages = [
    pkgs.gh
    pkgs.jq
  ];

  languages.rust.enable = true;

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

    "ghverify:bench" = {
      description = "Run benchmarks and generate report";
      exec = ''
        set -euo pipefail
        cargo build --release --bin gh-verify-bench -p gh-verify
        "$DEVENV_ROOT/target/release/gh-verify-bench"
      '';
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

    "ghverify:docs" = {
      description = "Generate rule specification docs from tests and Creusot specs";
      exec = ''
        set -euo pipefail
        cargo build --release -p gen-docs
        "$DEVENV_ROOT/target/release/gen-docs"
      '';
    };

    "ghverify:verify" = {
      description = "Creusot formal verification: translate + prove all predicates";
      exec = ''"$DEVENV_ROOT/scripts/verify.sh"'';
    };

    "ghverify:verify-one" = {
      description = "Prove a single Creusot predicate (pass name as argument)";
      exec = ''"$DEVENV_ROOT/scripts/verify.sh" "''${1:?Usage: devenv tasks run ghverify:verify-one <predicate_name>}"'';
    };
  };

  enterShell = ''
    echo "ghverify dev environment"
    echo "  rust: $(rustc --version)"
    echo "  gh  : $(gh --version | head -1)"
  '';
}

# ghverify - GitHub SDLC Verifier

SLSA-based GitHub SDLC health checker. Runs as a `gh` CLI extension, built in Rust.
Core verification logic is formally proven with Creusot + SMT solvers.

## Commands

```bash
cargo build                                    # Debug build
cargo build --release -p gh-verify             # Release build
cargo test --workspace                         # Run all tests
./target/release/gh-verify pr 123 --repo o/r   # PR verify
./target/release/gh-verify pr 123 --format json # JSON output
./target/release/gh-verify pr list-rules       # List rules
```

## Architecture

Three-crate workspace:

- `gh-verify-core` — pure runtime logic (serde only)
- `gh-verify` — CLI with I/O (reqwest, clap, tree-sitter)
- `gh-verify-verif` — Creusot verification targets (creusot-std only)

### gh-verify-core (crates/core/)

Pure verification logic. No I/O, no unsafe.

| Module | Purpose |
|--------|---------|
| `verdict.rs` | Severity enum, RuleResult type |
| `integrity.rs` | SLSA release checks (signatures, mutual approval, PR coverage) |
| `scope.rs` | PR scope classification by connected components |
| `union_find.rs` | Disjoint set union for call graph connectivity |

### gh-verify-verif (crates/verif/)

Creusot verification targets. Core predicates with `#[ensures]` specs
in a crate free of Creusot-unsupported constructs (`format!`, `String`, `Vec`).
Runtime implementations in `gh-verify-core` must match these verified predicates.

### gh-verify (crates/cli/)

I/O layer. Delegates all judgments to core.

| Change | File to create | Registration |
|---|---|---|
| New rule | `crates/cli/src/rules/<name>.rs` + impl `Rule` trait | Add to `engine.rs` `run_all` Vec |
| New subcommand | Add variant to `Commands` enum in `main.rs` | clap handles dispatch |
| New output format | `crates/cli/src/output/<name>.rs` | Add case in `output/mod.rs` |
| New API endpoint | `crates/cli/src/github/<name>.rs` | None |

## Formal Verification with Creusot

### Setup

```bash
# 1. Install dependencies
brew install opam z3
opam init --bare
opam switch create creusot 4.14.2
eval $(opam env --switch=creusot)
opam install alt-ergo why3 why3find

# 2. Install Creusot toolchain
cargo install --git https://github.com/creusot-rs/creusot cargo-creusot
NIGHTLY=$(cargo creusot version 2>&1 | grep 'Rust toolchain' | awk '{print $3}')
rustup toolchain install "$NIGHTLY"
rustup component add rustc-dev --toolchain "$NIGHTLY"
cargo +"$NIGHTLY" install --git https://github.com/creusot-rs/creusot creusot-rustc

# 3. Link binaries to Creusot's expected paths
CREUSOT_BIN="$HOME/Library/Application Support/creusot.creusot/bin"
mkdir -p "$CREUSOT_BIN"
for cmd in why3 why3find alt-ergo; do
  ln -sf "$(which $cmd)" "$CREUSOT_BIN/$cmd"
done
ln -sf "$(which z3)" "$CREUSOT_BIN/z3"

# 4. Generate Why3 config and prelude
cargo creusot why3-conf
CREUSOT_SRC=$(cargo metadata --format-version=1 | jq -r '.packages[] | select(.name=="creusot-std") | .manifest_path' | xargs dirname | xargs dirname)
cargo +"$NIGHTLY" run --manifest-path "$CREUSOT_SRC/prelude-generator/Cargo.toml"
DEST="$HOME/Library/Application Support/creusot.creusot/share/why3find/packages/creusot/creusot"
mkdir -p "$DEST"
cp "$CREUSOT_SRC/target/creusot/packages/creusot/creusot/"*.coma "$DEST/"
```

### Usage

```bash
eval $(opam env --switch=creusot)

# Translate Rust → Why3 (.coma files)
cargo creusot -p gh-verify-verif

# Prove a function
cargo creusot prove "<function_name>" -- -p gh-verify-verif
```

### Adding a new verified predicate

1. Add a pure function with `#[ensures]` to `crates/verif/src/lib.rs`
2. Use only primitive types (`bool`, `usize`) and the local `Severity` enum
3. Run `cargo creusot prove "<name>" -- -p gh-verify-verif`
4. Ensure `✔` before merging
5. Add the corresponding runtime function to `crates/core/` that delegates to the same logic

### Known issues

- Full crate proof (`cargo creusot prove`) may fail on `Debug` derive artifacts. Use per-function patterns.
- `cvc4` shim may fail calibration. `why3find.json` defaults to `alt-ergo`.
- `creusot-std` must be from git, not crates.io — version must match `creusot-rustc`.

### Design constraints

- **No `format!`/`String`/`Vec`** in verif crate — Creusot cannot translate these
- **`DeepModel` derive** required on enums used in `#[ensures]` comparisons
- **Primitive types only** — extract `bool`/`usize` predicates from complex functions
- **Severity enum duplicated** in verif crate to avoid pulling serde

## Naming

- Rule ID: kebab-case (`detect-unscoped-change`)
- File name: snake_case (`detect_unscoped_change.rs`)
- Crate name: kebab-case (`gh-verify-core`)

## Exit Codes

- `0`: all rules pass / warnings only
- `1`: one or more rules returned error

## PR Template

```markdown
## What
## Why
## How
## Verification
- [ ] `cargo test --workspace` passes
- [ ] Existing rules still work
- [ ] For new rules: verified pass/warning/error cases
- [ ] `--format json` output is valid JSON
- [ ] `cargo creusot prove` passes for affected predicates in verif crate
```

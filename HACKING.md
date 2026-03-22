# Hacking on ghverify

## Setup

```bash
# devenv (recommended: provides rust / gh / jq automatically)
direnv allow   # or: devenv shell
```

## Development

All commands are devenv tasks:

```bash
devenv tasks run ghverify:build          # Release build
devenv tasks run ghverify:test           # Unit + integration tests (no network)
devenv tasks run ghverify:bench          # Benchmarks (uses GitHub API)
devenv tasks run ghverify:dist           # Build release binary
devenv tasks run ghverify:fmt            # Format + clippy lint
devenv tasks run ghverify:verify         # Creusot formal verification
```

## Adding a Control

### SLSA control

1. Add `ControlId` variant in `crates/core/src/control.rs` (with `as_str` + `from_str`)
2. Map it in `crates/core/src/slsa.rs` (`control_slsa_mapping` + `ALL_SLSA_CONTROLS`)
3. Add integrity predicate in `crates/core/src/integrity.rs`
4. Add Creusot spec in `crates/verif/src/lib.rs` (matching integrity signature)
5. Create `crates/core/src/controls/<name>.rs` — implement `Control` trait, delegate to integrity predicate
6. Add to `crates/core/src/controls/mod.rs` (`instantiate()` match arm)

### Dev quality control

1. Add `ControlId` variant in `crates/core/src/control.rs`
2. Create `crates/core/src/controls/<name>.rs` — implement `Control` trait
3. Add to `crates/core/src/controls/mod.rs` (`instantiate()` + `compliance_controls()`)

## Adding a Subcommand

1. Add a variant to the `Commands` enum in `crates/cli/src/main.rs`
2. Handle it in the `run()` function

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
# Via devenv task (recommended)
devenv tasks run ghverify:verify

# Single predicate
devenv tasks run ghverify:verify-one <predicate_name>

# Or manually
eval $(opam env --switch=creusot)
cargo creusot -p gh-verify-verif
cargo creusot prove '<predicate_name>' -- -p gh-verify-verif
```

### Adding a new verified predicate

1. Add a pure function with `#[ensures]` to `crates/verif/src/lib.rs`
2. Use only primitive types (`bool`, `usize`) and the local `Severity` enum
3. Run `cargo creusot prove "<name>" -- -p gh-verify-verif`
4. Ensure pass before merging
5. Add the corresponding runtime function to `crates/core/` that delegates to the same logic

### Design constraints

- **No `format!`/`String`/`Vec`** in verif crate — Creusot cannot translate these
- **`DeepModel` derive** required on enums used in `#[ensures]` comparisons
- **Primitive types only** — extract `bool`/`usize` predicates from complex functions
- **Severity enum duplicated** in verif crate to avoid pulling serde

### Known issues

- Full crate proof (`cargo creusot prove`) may fail on `Debug` derive artifacts. Use per-function patterns.
- `cvc4` shim may fail calibration. `why3find.json` defaults to `alt-ergo`.
- `creusot-std` must be from git, not crates.io — version must match `creusot-rustc`.

## Release

```bash
git tag v0.2.0
git push origin v0.2.0
# → GitHub Actions builds and releases
```

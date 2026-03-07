# Hacking on ghlint

## Setup

```bash
# Nix flake (recommended: provides zig / gh / jq automatically)
direnv allow   # or: nix develop

# Manual: install Zig 0.15.x, GitHub CLI, jq
```

## Build & Test

```bash
build          # zig build -Doptimize=ReleaseSafe
lint-test      # unit tests (no network required)
lint-classify  # display domain classification table
lint-bench     # benchmarks (uses GitHub API)
```

## Adding a Rule

1. Create `src/rules/<name>.zig` — export `pub fn run(alloc, ctx) ![]RuleResult`
2. Add one line to the `rules` array in `src/rules/engine.zig`

## Adding a Subcommand

1. Create `src/cli/<name>.zig` — export `pub fn run(alloc, cfg, args) !void`
2. Add one line to the `dispatch_table` in `src/main.zig`

## Release

```bash
git tag v0.2.0
git push origin v0.2.0
# → GitHub Actions builds and releases for all platforms
```

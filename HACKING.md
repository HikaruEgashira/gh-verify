# Hacking on ghverify

## Setup

```bash
# devenv (recommended: provides zig / gh / jq automatically)
direnv allow   # or: devenv shell
```

## Tasks

All development tasks are managed via devenv. Run with `devenv tasks run <task>`.

```bash
devenv tasks run ghverify:build      # Release build
devenv tasks run ghverify:test       # Unit tests (no network required)
devenv tasks run ghverify:classify   # Display domain classification table
devenv tasks run ghverify:bench      # Benchmarks (uses GitHub API)
devenv tasks run ghverify:dist       # Cross-compile for all platforms
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

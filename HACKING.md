# Hacking on ghlint

This document covers the development workflow for contributors.

## Prerequisites

The easiest way to get a working environment is with [Nix]:

```bash
# With direnv (recommended)
direnv allow

# Or manually
nix develop
```

Without Nix, install directly:
- [Zig 0.15.x](https://ziglang.org/download/)
- [GitHub CLI](https://cli.github.com/)
- `jq`

## Build

```bash
zig build                          # debug build
zig build -Doptimize=ReleaseSafe   # release build
```

Output: `zig-out/bin/gh-lint`

## Tests

```bash
bash tests/run.sh                  # all tests
bash tests/run.sh --filter version # filtered
```

## Adding a Rule

1. Create `src/rules/<name>.zig` — export `pub fn run(alloc, ctx) ![]RuleResult`
2. Add one line to `src/rules/engine.zig` — append to the `rules` array
3. Add a test case in `tests/`
4. Document the rule in `README.md`

The rule file is self-contained; no other files need to change.

## Adding a Subcommand

1. Create `src/cli/<name>.zig` — export `pub fn run(alloc, cfg, args) !void`
2. Add one line to `src/main.zig` — append to `dispatch_table`
3. Add a GitHub Action under `action/<name>/` if it integrates with CI

## Release Process

Uses semantic versioning. To cut a release:

1. Update the version string in `src/main.zig`
2. Update `build.zig.zon` version field
3. Push a `v*` tag — the release workflow builds cross-platform binaries

```bash
git tag v0.2.0
git push origin v0.2.0
```

[Nix]: https://nixos.org/download

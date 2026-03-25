# Hacking on gh-verify

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
```

## Adding a Subcommand

1. Add a variant to the `Commands` enum in `main.rs`
2. Handle it in the `run()` function

## Adding a Control / Policy

Controls, policies, and evidence adapters live in [libverify](https://github.com/HikaruEgashira/libverify).
See the libverify HACKING guide for details.

## Release

```bash
git tag v0.2.0
git push origin v0.2.0
# → GitHub Actions builds and releases
```

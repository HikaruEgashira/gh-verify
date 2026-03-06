# Agent Guidelines for ghlint

This document describes how AI agents (Claude, Copilot, etc.) should contribute to ghlint.

## Architecture Invariants

Read `CLAUDE.md` for the full architecture. The key rule: **adding a feature never modifies existing files beyond a one-line registration**.

| Change type | Files to modify |
|---|---|
| New rule | Create `src/rules/<name>.zig`; add 1 line to `src/rules/engine.zig` |
| New subcommand | Create `src/cli/<name>.zig`; add 1 line to `src/main.zig` |
| New output format | Create `src/output/<name>.zig`; add 1 line to `src/output/formatter.zig` |

Do not refactor existing files while adding features.

## Build and Test

```bash
zig build
bash tests/run.sh
```

All tests must pass before opening a PR.

## Zig 0.15 Patterns

Common pitfalls in Zig 0.15 that differ from earlier versions:

- `ArrayList` is now unmanaged: use `var list: std.ArrayList(T) = .empty`, pass allocator per-op
- `std.io.getStdOut()` removed: use `std.fs.File.stdout()`
- `std.json.stringify` removed: use `std.json.Stringify.valueAlloc()`
- HTTP: use `request()` → `sendBodiless()` → `receiveHead()` → `reader()` → `appendRemainingUnlimited()`
- Disable gzip: set `accept_encoding: .{ .override = "identity" }` in request headers
- `build.zig` uses `root_module` not `root_source_file`

## Commit Scope

Keep commits and PRs scoped to a single domain. ghlint's `detect-unscoped-change` rule will flag PRs that touch unrelated areas — follow the same discipline in contributions here.

# ghlint - GitHub SDLC Linter

SLSA-based GitHub SDLC health checker. Runs as a `gh` CLI extension, built in Zig.

## Commands

```bash
zig build                                    # Debug build
zig build -Doptimize=ReleaseSafe             # Release build
./zig-out/bin/gh-lint pr 123 --repo o/r      # PR lint
./zig-out/bin/gh-lint pr 123 --format json   # JSON output
./zig-out/bin/gh-lint pr list-rules          # List rules
```

## Architecture (Open/Closed Principle)

Extensions require only a new file and one line of registration. No changes to existing logic.

| Change | File to create | Registration |
|---|---|---|
| New rule | `src/rules/<name>.zig` | 1 line in `engine.zig` `rules` array |
| New subcommand | `src/cli/<name>.zig` | 1 line in `main.zig` `dispatch_table` |
| New output format | `src/output/<name>.zig` | 1 case in `formatter.zig` switch |
| New API endpoint | `src/github/<name>.zig` | None |

## Zig 0.15 Conventions

```zig
// ArrayList: Unmanaged (pass alloc to each operation)
var list: std.ArrayList(T) = .empty;
try list.append(alloc, item);

// stdout/stderr
const stdout = std.fs.File.stdout().deprecatedWriter();

// HTTP: request API (not fetch)
var req = try client.request(.GET, uri, .{
    .headers = .{ .accept_encoding = .{ .override = "identity" } },
    .extra_headers = &[_]std.http.Header{...},
});
try req.sendBodiless();
var response = try req.receiveHead(&redirect_buf);
const reader = response.reader(&transfer_buf);
try reader.appendRemainingUnlimited(alloc, &body);

// JSON
const parsed = try std.json.parseFromSlice(T, alloc, body, .{ .ignore_unknown_fields = true });
const json_str = try std.json.Stringify.valueAlloc(alloc, value, .{ .whitespace = .indent_2 });

// build.zig: root_module + createModule (not direct root_source_file)
```

## Naming

- Rule ID: kebab-case (`detect-unscoped-change`)
- File name: snake_case (`detect_unscoped_change.zig`)
- Zig reserved-word enum variants: `@"test"`, `@"error"`

## Exit Codes

- `0`: all rules pass / warnings only
- `1`: one or more rules returned error

## Dependencies

- tree-sitter (core, go, python): Homebrew
- tree-sitter-typescript: vendored in `deps/`

## PR Template

```markdown
## What
## Why
## How
## Verification
- [ ] `zig build` succeeds
- [ ] Existing rules still work
- [ ] For new rules: verified pass/warning/error cases
- [ ] `--format json` output is valid JSON
```

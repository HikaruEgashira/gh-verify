# ghlint - GitHub SDLC Linter

SLSA ベースの GitHub SDLC 健全性検証ツール。gh CLI 拡張として動作する Zig 製バイナリ。

## Commands

```bash
zig build                                    # Debug ビルド
zig build -Doptimize=ReleaseSafe             # リリースビルド
./zig-out/bin/gh-lint pr 123 --repo o/r      # PR lint
./zig-out/bin/gh-lint pr 123 --format json   # JSON 出力
./zig-out/bin/gh-lint pr list-rules          # ルール一覧
```

## Architecture (Open/Closed Principle)

拡張は「新規ファイル追加 + 登録1行」で完結する。既存ファイルのロジック変更を伴わない。

| 変更 | 作成ファイル | 登録 |
|---|---|---|
| 新ルール | `src/rules/<name>.zig` | `engine.zig` の `rules` 配列に1行 |
| 新サブコマンド | `src/cli/<name>.zig` | `main.zig` の `dispatch_table` に1行 |
| 新出力フォーマット | `src/output/<name>.zig` | `formatter.zig` の switch に1行 |
| 新APIエンドポイント | `src/github/<name>.zig` | 不要 |

## Zig 0.15 Conventions

```zig
// ArrayList: Unmanaged（alloc を各操作で渡す）
var list: std.ArrayList(T) = .empty;
try list.append(alloc, item);

// stdout/stderr
const stdout = std.fs.File.stdout().deprecatedWriter();

// HTTP: request API（fetch ではない）
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

// build.zig: root_module + createModule（root_source_file 直接指定ではない）
```

## Naming

- ルール ID: kebab-case (`detect-unscoped-change`)
- ファイル名: snake_case (`detect_unscoped_change.zig`)
- Zig 予約語の enum variant: `@"test"`, `@"error"`

## Exit Codes

- `0`: 全ルール pass / warning のみ
- `1`: いずれかのルールが error

## Dependencies

- tree-sitter (core, go, python): Homebrew
- tree-sitter-typescript: `deps/` にベンダリング

## PR Template

```markdown
## What
## Why
## How
## Verification
- [ ] `zig build` が成功する
- [ ] 既存ルールが引き続き動作する
- [ ] 新ルールの場合: pass/warning/error の3パターンを確認
- [ ] `--format json` 出力が valid JSON
```

# ghlint - GitHub SDLC Linter

SLSA ベースの GitHub SDLC 健全性検証ツール。gh CLI 拡張として動作する Zig 製バイナリ。

## Common Commands

```bash
zig build                                    # Debug ビルド
zig build -Doptimize=ReleaseSafe             # リリースビルド
./zig-out/bin/gh-lint pr 123 --repo o/r      # PR lint 実行
./zig-out/bin/gh-lint pr 123 --format json   # JSON 出力
./zig-out/bin/gh-lint pr list-rules          # ルール一覧
GH_TOKEN=$(gh auth token) ./zig-out/bin/gh-lint pr 123 --repo o/r  # トークン明示指定
```

## Architecture (Open/Closed Principle)

拡張は「新規ファイル追加 + 登録1行」で完結する。既存ファイルのロジック変更を伴わない。

| 変更 | 作成ファイル | 登録 |
|---|---|---|
| 新ルール | `src/rules/<name>.zig` | `engine.zig` の `rules` 配列に1行 |
| 新サブコマンド | `src/cli/<name>.zig` | `main.zig` の `dispatch_table` に1行 |
| 新APIエンドポイント | `src/github/<name>.zig` | 不要 |
| 新出力フォーマット | `src/output/<name>.zig` | `formatter.zig` の switch に1行 |

### Layer Responsibilities

- `build.zig` — バイナリ名 `gh-lint` の設定のみ。ルール追加で変更しない
- `src/main.zig` — env vars → Config, dispatch_table → サブコマンド委譲。ロジックなし
- `src/cli/` — 引数解析 + 各レイヤーへの委譲のみ。ルールロジックを含まない
- `src/github/` — HTTP client (`client.zig`) + エンドポイント別モジュール。`client.zig` は `[]const u8` を返すだけ
- `src/rules/rule.zig` — `RuleFn`, `RuleResult`, `RuleContext` 型定義のみ。変更しない
- `src/rules/engine.zig` — ルール登録配列 + `runAll` 集約。新ルール追加時のみ1行変更
- `src/rules/<name>.zig` — 個別ルール実装。`pub fn run(alloc, ctx) ![]RuleResult` を export
- `src/output/` — `formatter.zig` が format flag で振り分け。各 fmt は `[]RuleResult` → stdout
- `src/util/` — 純粋関数のみ（副作用なし）

## Code Style

### Zig 0.15 Conventions

```zig
// ArrayList: Unmanaged スタイル（alloc を各操作で渡す）
var list: std.ArrayList(T) = .empty;
try list.append(alloc, item);
const slice = try list.toOwnedSlice(alloc);

// stdout/stderr
const stdout = std.fs.File.stdout().deprecatedWriter();
try stdout.print("format {s}\n", .{arg});

// HTTP client: request API を使用（fetch ではない）
var req = try client.request(.GET, uri, .{
    .headers = .{ .accept_encoding = .{ .override = "identity" } },
    .extra_headers = &[_]std.http.Header{...},
});
try req.sendBodiless();
var response = try req.receiveHead(&redirect_buf);
const reader = response.reader(&transfer_buf);
try reader.appendRemainingUnlimited(alloc, &body);

// JSON parse
const parsed = try std.json.parseFromSlice(T, alloc, body, .{ .ignore_unknown_fields = true });
defer parsed.deinit();

// JSON serialize
const json_str = try std.json.Stringify.valueAlloc(alloc, value, .{ .whitespace = .indent_2 });
```

### Naming

- ルール ID: kebab-case (`detect-unscoped-change`)
- ファイル名: snake_case (`detect_unscoped_change.zig`)
- Zig 予約語の enum variant: `@"test"`, `@"error"` でエスケープ

## Output Design

### Human Format

```
[rule-id] pass|warning|error: message
  domain (N lines): file1 file2
  Suggestion: ...
```

- ANSI カラー: pass=GREEN, warning=YELLOW, error=RED, rule-id=BOLD
- JSON format: `--format json` で `[{ rule_id, severity, message, affected_files, suggestion }]`

### Exit Codes

- `0`: 全ルール pass または warning のみ
- `1`: いずれかのルールが error

## State Management

- **Config**: `main.zig` の `Config` struct で一元管理。env vars → struct → 全レイヤーに渡す
- **Token 解決順**: `GH_TOKEN` → `GH_ENTERPRISE_TOKEN` → `gh auth token` フォールバック
- **メモリ**: `GeneralPurposeAllocator` をルートで確保。Debug ビルドでリーク検出

## Error Handling

- HTTP エラー: `error.HttpError` を返し、CLI 層で stderr メッセージ + `exit(1)`
- JSON パースエラー: `error.SyntaxError` として上位に伝播
- 引数エラー: stderr にヘルプを出力して `exit(1)`
- ルール内エラー: Zig の error union で `anyerror` 伝播。engine が集約

## Debugging

```bash
# Debug ビルドは GPA がメモリリークを検出する
zig build && ./zig-out/bin/gh-lint pr 123 --repo o/r

# HTTP レスポンスのデバッグ（client.zig に一時追加）
std.debug.print("[DEBUG] ({d} bytes): {s}\n", .{ result.len, result[0..@min(200, result.len)] });

# GitHub API を直接叩いて比較
curl -s -H "Authorization: Bearer $(gh auth token)" \
  "https://api.github.com/repos/OWNER/REPO/pulls/N/files" | jq .
```

## Pull Request Template

PRは以下のテンプレートに従う:

```markdown
## What

<!-- 変更の概要を1-2文で -->

## Why

<!-- SLSA のどのレベル/原則に基づくか、どの SDLC 工程を検証するか -->

## How

<!-- 技術的なアプローチ。新ルールの場合はアルゴリズムの概要 -->

## Verification

- [ ] `zig build` が成功する
- [ ] 既存ルールが引き続き動作する（`gh lint pr <known-PR> --repo <repo>`）
- [ ] 新ルールの場合: pass/warning/error の3パターンを確認
- [ ] `--format json` 出力が valid JSON
```

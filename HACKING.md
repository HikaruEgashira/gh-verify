# Hacking on ghlint

## Setup

```bash
# devenv (推奨: zig / gh / jq が自動で揃う)
direnv allow   # または: devenv shell

# 手動インストール: Zig 0.15.x, GitHub CLI, jq
```

## Build & Test

```bash
build          # zig build -Doptimize=ReleaseSafe
lint-test      # ユニットテスト (ネット不要)
lint-classify  # ドメイン分類テーブル表示
lint-bench     # ベンチマーク (GitHub API 使用)
```

## Adding a Rule

1. `src/rules/<name>.zig` を作成 — `pub fn run(alloc, ctx) ![]RuleResult` を export
2. `src/rules/engine.zig` の `rules` 配列に1行追加

## Adding a Subcommand

1. `src/cli/<name>.zig` を作成 — `pub fn run(alloc, cfg, args) !void` を export
2. `src/main.zig` の `dispatch_table` に1行追加

## Release

```bash
git tag v0.2.0
git push origin v0.2.0
# → GitHub Actions が devenv で全プラットフォームをビルド・リリース
```

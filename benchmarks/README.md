# ghlint ベンチマーク

実世界の GitHub PR を対象に ghlint の `detect-unscoped-change` ルールを検証するベンチマーク群です。

## 実行方法

```bash
# ghlint をビルド
cd ..
zig build

# ベンチマーク実行
cd benchmarks
GHLINT_BIN=../zig-out/bin/gh-lint bash run.sh
```

結果は `results/run_<timestamp>.json` に保存されます。

## ケース構成

| カテゴリ | 件数 | 説明 |
|--------|------|------|
| pass/  | 7    | 単一ドメインまたは正当なスコープのPR |
| warn/  | 4    | 2つの無関係ドメインを跨ぐPR |
| error/ | 3    | 3つ以上のドメインを跨ぐPR |

## ドメイン分類ルール（`src/util/diff_parser.zig`）

| ドメイン | 検出パターン |
|---------|------------|
| `test`  | パスに "test"/"spec" を含む、または `_test.*`/`.spec.*`/`.test.*` |
| `ci`    | `.github/` で始まる、または "ci"/"workflow" を含む |
| `docs`  | `docs/` で始まる、または `.md`/`.rst`/`.txt` で終わる |
| `auth`  | "auth"/"login"/"token"/"session"/"oauth" を含む |
| `database` | "db"/"database"/"migration"/"schema" を含む、または `.sql` |
| `ui`    | "ui"/"component"/"view"/"page" を含む、または `.css`/`.scss`/`.tsx`/`.jsx` |
| `api`   | "api"/"handler"/"route"/"controller"/"endpoint" を含む |
| `config`| "config" を含む、または `.toml`/`.yaml`/`.yml`/`.env` で終わる |
| `unknown` | 上記のどれにも一致しない（カウントから除外） |

**優先順位**: 上から順に最初に一致したドメインが採用される。

**特別ルール**:
- `test` ドメインは常に無視される（カウント対象外）
- `unknown` ドメインも無視される
- ノイズ閾値: ドメインあたり5行以下は無視
- PASS 条件: ドメイン数 ≤1、または ドメイン数が2かつ一方が `docs`
- WARN 条件: 無関係ドメイン数 = 2
- ERROR 条件: 無関係ドメイン数 ≥ 3

## 発見された既知の問題（False Positive / False Negative）

### False Positive（誤検知）

| ケース | 問題 | 例 |
|--------|------|-----|
| 部分文字列マッチ | `token_parser` が "token" を含むため `auth` ドメインに分類される | `stripe-sessions-contest.tsx` → `auth` |
| `schema` の過剰マッチ | Zod スキーマや JSON スキーマが `database` ドメインに分類される | `schemas.ts` → `database` |
| `session` の過剰マッチ | カンファレンスセッション等が `auth` ドメインに分類される | `stripe-sessions-contest.tsx` |

### False Negative（見逃し）

| ケース | 問題 | 例 |
|--------|------|-----|
| `package.json` | `.json` はどのパターンにもマッチせず `unknown` として無視 | 依存関係バンプPRが常にPASS |
| `Dockerfile` | `unknown` に分類され無視される | 大規模インフラ変更でもPASS |
| 大文字ディレクトリ | case-sensitive マッチのため `Auth/` が `auth` と一致しない | Java/Kotlinリポジトリ |
| `.go`/`.rs`/`.py` | 標準的なソースファイルがドメインパスを使わない場合 `unknown` に | 汎用Goリポジトリ全体 |

## ケース選定基準

1. **実行可能性**: 公開リポジトリの Merged PR のみ
2. **確認済み**: `ghlint pr <num> --repo <owner>/<repo> --format json` で実際に検証済み
3. **多様性**: 複数のエコシステム（TypeScript, Python, JavaScript）
4. **教育的価値**: false positive/negative を含む興味深いケースを優先

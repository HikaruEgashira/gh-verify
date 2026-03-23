# gh-verify Positioning

競合ベンダー調査に基づくgh-verifyの位置づけ。
各ベンダーの詳細は [vendors/](./vendors/) を参照。

## 戦略

gh-verifyは **GitHubリソースの検証に専念するCLIツール** である。
`gh` CLI拡張として動作し、GitHub以外のプラットフォームには依存しない。

## 現在の実装規模

| 項目 | 数量 |
|------|------|
| コントロール | 20 (SLSA Source×5, Build×5, SOC2 CC7/CC8×10) |
| 出力形式 | 3 (human, JSON, SARIF) |
| ポリシープリセット | 5 (default, oss, aiops, soc1, soc2) |
| CLIサブコマンド | 2 (pr, release) |
| Creusot検証済み述語 | 15+ |

## 競合一覧

| ベンダー | 調査ファイル |
|----------|-------------|
| Aviator | [aviator.md](./vendors/aviator.md) |
| JFrog Evidence | [jfrog_evidence.md](./vendors/jfrog_evidence.md) |

## 設計方針

- **保管はGitHubに寄せる**: attestation保管、証跡保管等のストレージ機能は自前で持たず、GitHubの既存機能 (Artifact Attestations, Actions Artifacts等) に委譲する
- **GitHubにできない検証を提供する**: gh-verifyの価値は保管ではなく、GitHubが提供しない検査ロジックにある
- **GitHub以外のプラットフォームには依存しない**: Jira等の外部ツール連携はgh-verifyのスコープ外。別verifierとしてエコシステムに配置する

## gh-verify の独自性

1. **形式検証されたルールロジック** — Creusot + SMTで判定ロジック自体の正しさを数学的に証明。15+述語を検証済み。競合にこの層はない
2. **lint的パラダイム + ポリシーエンジン** — 20コントロールを決定的・ヒューリスティックに高速実行。OPA Regoによるポリシーエンジンで組織ごとの判定カスタマイズが可能 (5プリセット同梱)
3. **SLSA v1.2フレームワーク準拠** — Source Track L0〜L4, Build Track L0〜L3を体系的にカバー。PR検証とリリース検証の両方を単一ツールで提供
4. **SOC2コンプライアンスマッピング** — CC7.1/CC7.2/CC8.1にマッピングされた10コントロール。SOC1/SOC2ポリシープリセットで判定基準を切り替え可能
5. **CI/CDネイティブ統合** — SARIF出力でGitHub Code Scanning等と直接統合。バッチ検証 (PR範囲・SHA範囲・タグ範囲・日付範囲) で監査的な一括検証が可能
6. **構造化出力** — JSON/SARIF出力により、外部ツールやCI/CDパイプラインとの統合が容易

## 競合との軸の違い

| 軸 | gh-verify | 競合 |
|----|-----------|------|
| アーキテクチャ | gh CLI拡張 (単一バイナリ) | モノリシックSaaS |
| 正しさの保証対象 | ルールロジック自体 (形式検証) | データの改竄防止 (暗号署名) / プロセスの強制 |
| 保管 | GitHubに委譲 (自前で持たない) | 独自プラットフォームに集約 |
| 検査スタイル | lint的 + OPA Regoポリシーエンジン | ワークフロー自動化 / 証跡収集 |
| カバー範囲 | GitHub PR + Release検証 | PRマージ〜デプロイ中心 |
| コンプライアンス | SOC1/SOC2プリセット同梱、CC7/CC8マッピング済み | 独自フレームワーク or なし |
| GitHubとの関係 | GitHubの検証能力を**補完** | GitHubを**置換**または**ラップ** |

## スコープ外

| 機能 | 理由 |
|------|------|
| GitHub外ツール連携 (Jira, Confluence等) | 1 tool = 1 platform。別ツールの責務 |
| SOCレポート文書生成 | CLIの責務ではない。出力を消費する側が担う |
| ダッシュボード / Web UI | 同上 |

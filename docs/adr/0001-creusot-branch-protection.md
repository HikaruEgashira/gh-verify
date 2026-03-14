# ADR-0001: GitHub branch protection の bypass semantics を Creusot で表現する

## ステータス
採用

## コンテキスト

GitHub の branch protection は REST 応答がネストしており、`documented bypass があるか` と
`厳格運用として十分か` という 2 つの関心が API 形状と混ざりやすい。

shell や `jq` で直接判定すると、次の問題がある。

- bypass semantics が散逸する
- `strict` と `bypass` が同じ if 文に潰れてしまう
- GitHub 側のレスポンス差分に引きずられて仕様モデルまで不明瞭になる

## 決定

branch protection の検査は 3 層に分離する。

1. GitHub REST 応答
2. 正規化した `BranchProtectionSpec`
3. Creusot の型不変条件を持つ `StrictBranchProtection` と bypass model

`StrictBranchProtection` は hardening profile を保持する。
別途、documented bypass surface と `authorized change => requirements satisfied` の関係を
Creusot の論理関数で保持する。
CLI は JSON 入力を正規化して bypass surface と hardening の両方を表示するだけに留める。

rulesets については black-box differential で documented semantics を補強する。
特に `RepositoryRole` bypass は external collaborator には効かず、org member に対してのみ
機能するものとしてモデル化する。

## 理由

- `Consolidation of Meaning`: 厳格性の定義を 1 モジュールへ集約できる
- `Interface Realignment`: API 応答と業務ルールを切り離せる
- `Inversion of Control`: CLI や将来の GitHub Actions 連携は、検証済みライブラリに依存するだけでよい

## リスクと対応

- リスク: GitHub API の応答形状が増える
- 対応（軽減）: 正規化境界を `main.rs` に限定し、証明対象の `src/lib.rs` を安定化する

- リスク: `strict` の定義が厳しすぎて運用に合わない
- 対応（軽減）: `BranchProtectionSpec` は維持し、将来は profile を増やしても型境界を崩さない

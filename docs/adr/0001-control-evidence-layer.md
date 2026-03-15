# ADR-0001: Control and Evidence Layerを導入する

## ステータス
採用

## コンテキスト
現在の検証ロジックはI/Oからは分離されているが、意味論はGitHubの語彙に強く依存している。
`PR`、`Review`、`Release` といった platform 固有の概念がコア型へ露出しており、
内部統制と SLSA の評価対象である「証跡」と「統制」が明示されていない。

この構造では次の問題がある。

- API欠損と統制違反を区別しにくい
- GitHub以外の証跡ソースへ拡張しにくい
- rule が platform 表現と control 表現を同時に背負っている

## 決定
`gh-verify-core` に `evidence`、`control`、`controls`、`profile` の4レイヤを導入する。

- `evidence`: 正規化済み証跡の表現
- `control`: control ID、status、finding、評価 trait
- `controls`: SLSA / 内部統制ベースの control catalog
- `profile`: control finding を gate severity へ写像する方針

GitHub 固有の型と収集処理は adapter 層へ閉じ込める。

## 理由
- 意味の集約: `partial / missing / not_applicable` を `EvidenceState` に集約できる
- インターフェース整列: control は `Violated` と `Indeterminate` を区別できる
- 依存反転: platform DTO を core に流さず、adapter が正規化責務を持つ

## リスクと対応
- リスク: 新旧レイヤがしばらく併存する
- 対応（軽減）: 既存 rule を即時置換せず、adapter と control skeleton を先に並走させる

- リスク: abstraction が先行し、利用箇所が追随しない
- 対応（軽減）: review independence と source authenticity を最初の移行対象に固定する

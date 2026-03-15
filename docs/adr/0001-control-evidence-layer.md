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

互換層は永続化しない。
新旧移行のための互換コードが必要な場合は CLI 境界に限定し、core に入れない。

## 理由
- 意味の集約: `partial / missing / not_applicable` を `EvidenceState` に集約できる
- インターフェース整列: control は `Violated` と `Indeterminate` を区別できる
- 依存反転: platform DTO を core に流さず、adapter が正規化責務を持つ

## 移行原則
- 新機能は `rules` や GitHub DTO 直結の旧系へ追加しない
- 主系の実行経路は `adapter -> EvidenceBundle -> assessment -> profile` とする
- 互換コードは一方向変換のみを担い、判定ロジックを持たない
- adapter は旧 `rules` 型へ依存しない

## 互換層の寿命管理
- 互換層は migration seam であり、恒久 API として扱わない
- 互換層の配置先は CLI 境界配下に限定する
- `gh-verify-core` には `compat`、`legacy`、`github_*` のような移行専用モジュールを作らない
- 互換層に新しい control や評価ロジックを追加しない
- 互換層の責務は旧入力を新しい `EvidenceBundle` へ写像することだけとする
- 互換層を経由しない新主系の呼び出し入口を先に作り、その後に旧入口を削除する

## 実施順序
1. adapter が `EvidenceBundle` を構築する
2. `assessment` が control と profile を評価する
3. CLI が `assessment` を主系として呼ぶ
4. 旧 `rules` は薄い wrapper に縮退する
5. `RuleContext` と旧 rule 実装を削除する

## 禁止事項
- 旧 `rules` 側に新しい設定や分岐を追加する
- adapter から `RuleContext` や旧 rule 型を参照する
- profile 判定を CLI 出力コードへ再分散する
- `Indeterminate` を `Violated` や空集合へ潰す

## 削除条件
- `gh-verify-core` から GitHub 固有語彙が消えている
- CLI の評価入口が `RuleContext` ではなく `EvidenceBundle` になっている
- 互換コードが CLI 境界から削除されている
- 旧 rule 実装が新主系の wrapper すら持たず削除されている
- CI と Action が profile ベースの出力だけで成立している

## リスクと対応
- リスク: 新旧レイヤがしばらく併存する
- 対応（軽減）: 既存 rule を即時置換せず、adapter と control skeleton を先に並走させる

- リスク: abstraction が先行し、利用箇所が追随しない
- 対応（軽減）: review independence と source authenticity を最初の移行対象に固定する

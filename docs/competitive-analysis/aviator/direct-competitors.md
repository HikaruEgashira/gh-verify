# Aviator 直接競合

## 調査メタデータ

- 調査日: 2026-03-15
- 対象会社: Aviator
- 分類: 直接競合

## 対象範囲

この文書では、Aviator と同じく `stacked PR` と `merge queue` を主戦場に持つ会社だけを扱う。

## 分析軸

| 軸 | 観点 | 重要度 |
|----|------|--------|
| 開発フロー適合 | stacked PR を自然に扱えるか | 高 |
| マージ自動化 | merge queue, batch, CI 最適化の深さ | 高 |
| レビュー運用 | reviewer assignment, inbox, review UX の強さ | 高 |
| 導入摩擦 | GitHub 既存運用からの移行コスト | 中 |
| 価格 | 導入初期の seat 単価と無料枠 | 中 |

## 競合会社別の整理

### Graphite

最も近い直接競合である。Aviator と同様に stacked PR と merge queue を中核に据えるが、
Graphite は review UX と AI 機能まで前面に出している。

**強い点**

- Graphite CLI は stack 全体を submit する運用を前提にしている
- merge queue を自社製品として提供している
- review inbox, reviewer assignment, AI review, AI chat まで一体で提供する

**弱い点**

- Aviator と比べると価格が重い
- review 体験を Graphite 寄りに再編する前提が強く、導入時の運用変更が大きくなりやすい

**Aviator とのギャップ**

- `av` の直接競合としては最有力
- Graphite は review 面そのものを置き換える志向が強い
- Aviator は queue, ownership, release governance を比較的分離した形で導入しやすい

### Mergify

Merge automation では強い直接競合だが、stacked PR のモデルが Aviator と異なる。

**強い点**

- merge queue, batch, parallel checks, queue priority など CI 最適化が厚い
- monorepo 運用や自動化の文脈で導入しやすい
- `Depends-On:` による pull request dependency を持つ

**弱い点**

- review UI や review inbox は主戦場ではない
- stacks は branch stack ではなく `1 branch / 1 commit = 1 PR` の設計で、Aviator や Graphite と運用感が違う

**Aviator とのギャップ**

- merge queue と CI economics を優先する組織では Mergify が強い
- 小さな差分を review しやすく積み上げる開発体験では Aviator が優位

## ギャップマトリクス

| 領域 | Aviator | Graphite | Mergify |
|------|---------|----------|---------|
| Stacked PR 体験 | 強い | 非常に強い | 中 |
| Merge Queue | 強い | 強い | 非常に強い |
| Review UX | 中 | 非常に強い | 弱い |
| Reviewer Routing | 強い | 中 | 弱い |
| 導入の軽さ | 強い | 中 | 中 |
| 価格の軽さ | 強い | 弱い | 中 |

## 価格と導入条件

2026-03-15 時点の公開 pricing では、Aviator は初期導入の軽さで優位にある。

- Aviator MergeQueue は `Free under 15 users`、Pro は `$12 / user / month`
- Aviator の stacked PR CLI は OSS として公開されている
- Graphite Team は `$40 / user / month, billed annually` で Merge Queue を含む
- Mergify Max は `$21 / seat / month` で Merge Queue と Workflow Automation を含む

このため、stacked PR と merge queue を小さく始める比較では Aviator が有利である。

## 根本原因分析

### ギャップ1: Graphite の review UX が強い

| 層 | 問い | 回答 |
|----|------|------|
| 表層 | なぜ review UX で差が出るか | Graphite は inbox と review surface まで製品化している |
| 中層 | なぜそこまで広げるのか | stacked PR を review 体験ごと最適化したいから |
| 深層 | なぜ Aviator は差があるのか | Aviator は queue, ownership, release との接続を重視している |
| 根本 | 製品戦略の差 | Graphite は review-first、Aviator は workflow governance 寄り |

### ギャップ2: Mergify の CI 最適化が厚い

| 層 | 問い | 回答 |
|----|------|------|
| 表層 | なぜ merge queue の機能差が出るか | Mergify は merge automation を主戦場にしている |
| 中層 | なぜそこへ集中するのか | 大規模 CI と monorepo のコスト最適化が顧客価値だから |
| 深層 | なぜ Aviator は別の強みを持つのか | stack, reviewer routing, release 側の一貫性を重視している |
| 根本 | 出自の差 | Mergify は automation-first、Aviator は developer workflow suite |

## 判断指針

- stacked PR と merge queue を低コストで同時導入したいなら Aviator が第一候補
- review 体験や AI までまとめて置き換えたいなら Graphite が第一候補
- CI economics と merge automation を最優先するなら Mergify が第一候補

## 参照ソース

- [Aviator Pricing](https://www.aviator.co/pricing)
- [Aviator Stacked PRs](https://www.aviator.co/stacked-prs)
- [Aviator MergeQueue](https://www.aviator.co/merge-queue)
- [aviator-co/av GitHub](https://github.com/aviator-co/av)
- [Graphite Pricing](https://graphite.dev/pricing)
- [Graphite Create and Submit PRs](https://graphite.dev/docs/create-submit-prs)
- [Graphite Merge Queue](https://graphite.dev/docs/graphite-merge-queue)
- [Mergify Merge Queue](https://docs.mergify.com/merge-queue/)
- [Mergify Stacks](https://docs.mergify.com/stacks/)
- [Mergify Pricing](https://mergify.com/pricing)

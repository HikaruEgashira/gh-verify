# Aviator 競合分析

## 調査メタデータ

- 調査日: 2026-03-15
- 対象会社: Aviator
- 対象プロダクト: `av` CLI, Stacked PRs, MergeQueue, FlexReview, Pilot, Releases

## 対象会社の現在地

Aviator は単体の `av` CLI ベンダーではなく、GitHub 上の開発フロー全体を扱う
プラットフォームとして整理するのが自然である。

- `av` は GitHub 向けの stacked PR 運用を支える OSS CLI
- Stacked PRs は小さな差分を直列に積み上げるレビュー運用を支援する
- MergeQueue は mainline への投入順序と CI を制御する
- FlexReview は `CODEOWNERS` を超える reviewer routing を狙う
- Pilot と Releases まで含めると、Aviator は review-to-release の運用面を束ねる製品群になる

競合調査は責務ごとに次の 2 文書へ分離する。

- [直接競合](./direct-competitors.md)
- [周辺競合](./adjacent-competitors.md)

## 分類方針

- 直接競合:
  Aviator と同じく `stacked PR` と `merge queue` を主戦場に持つ会社
- 周辺競合:
  GitHub native, VCS 再設計, OSS 代替など、導入判断では比較対象になるが主戦場が完全一致しない会社や製品

## 速い結論

- 主戦場の直接競合は Graphite と Mergify
- 導入比較で必ず出る周辺競合は GitHub と Meta / Sapling
- OSS 代替としては `ghstack` も収集対象に残す

## 参照導線

- 主戦場比較を読む場合:
  [direct-competitors.md](./direct-competitors.md)
- 周辺比較を読む場合:
  [adjacent-competitors.md](./adjacent-competitors.md)

## 参照ソース

- [Aviator Pricing](https://www.aviator.co/pricing)
- [Aviator Stacked PRs](https://www.aviator.co/stacked-prs)
- [Aviator MergeQueue](https://www.aviator.co/merge-queue)
- [Aviator FlexReview](https://docs.aviator.co/flexreview)
- [aviator-co/av GitHub](https://github.com/aviator-co/av)

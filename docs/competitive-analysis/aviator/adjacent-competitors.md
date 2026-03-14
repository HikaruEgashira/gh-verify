# Aviator 周辺競合

## 調査メタデータ

- 調査日: 2026-03-15
- 対象会社: Aviator
- 分類: 周辺競合

## 対象範囲

この文書では、Aviator の主戦場と完全一致はしないが、導入判断では比較対象になった
周辺競合と代替を収集する。

## 収集した周辺競合

### GitHub

GitHub は追加 SaaS を入れない場合の基準線である。

**強い点**

- 既存の GitHub ワークフローにそのまま乗る
- merge queue を native に使える
- 組織内標準として通しやすい

**弱い点**

- stacked PR 運用の体験は弱い
- review routing, ownership, queue orchestration を横断して最適化する製品ではない

**Aviator との距離**

- SaaS 追加を避ける組織では GitHub が有力
- stack-aware な運用や reviewer routing を含めて最適化したい場合は Aviator の価値が出る

### Meta / Sapling

Sapling は stacked changes の思想としては強いが、Aviator の直接代替として見ると
導入コストの質が異なる。

**強い点**

- stacked changes のモデルが深い
- Git 運用そのものを改善する発想が強い

**弱い点**

- GitHub 標準運用との距離がある
- 導入は CLI 1 本の置き換えではなく、チームの作業モデル変更を伴いやすい

**Aviator との距離**

- Aviator は GitHub 前提の現実解
- Sapling は VCS ワークフロー自体の再設計に近い

### 会社に紐づかない OSS 代替

今回の周辺調査では、会社単位の競合だけでなく OSS 代替も比較対象に含めた。

#### ghstack

`ghstack` は stacked PR の OSS 代替として有力だが、通常の GitHub UI にそのまま馴染む
完成品ではない。Aviator と比較すると、製品ではなく低レベルな運用部品として見るべきである。

## 導入判断での位置づけ

- GitHub:
  SaaS 追加を避ける比較軸
- Meta / Sapling:
  VCS ワークフローを再設計する比較軸
- ghstack:
  OSS 部品で自前運用する比較軸

## 参照ソース

- [GitHub Merge Queue](https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/incorporating-changes-from-a-pull-request/merging-a-pull-request-with-a-merge-queue?tool=webui)
- [Sapling Stack](https://sapling-scm.com/docs/git/sapling-stack/)
- [ghstack](https://github.com/ezyang/ghstack)

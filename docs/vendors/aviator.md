# Aviator
https://github.com/aviator-co/av
https://www.aviator.co

## 会社の歴史

- 2021年4月創業。サンフランシスコ
- 創業者: **Ankit Jain** (CEO) と **Spriha Baruah Tucker**。両名とも元Google社員
- 創業の動機: Googleの社内開発者生産性ツール (マージキュー、コードレビュー最適化等) のレベルを全ての開発チームに提供する
  - "Google-level engineering productivity to every developer"
- **Y Combinator Summer 2021 (S21)** バッチ
- 社名の由来: 不明 (公式説明なし)

### 共同創業者の離脱
- Spriha Baruah Tucker は2021年4月〜2023年7月まで在籍後、BuildkiteにField CTOとして移籍
  - https://theorg.com/org/aviator/org-chart/spriha-baruah-tucker
  - https://www.linkedin.com/in/spriha-tucker/
- 現在は実質Ankit Jainのソロファウンダー体制

### チーム

**Ankit Jain** (CEO & Co-founder)
- Google, Adobe でエンジニア
- Sunshine, Homejoy, Shippo でエンジニアリングリード
- Unshackled Ventures で EIR (Entrepreneur in Residence)
- Xoogler.co (Google卒業生ネットワーク) の創設者
- DeveloperWeek, Monorepo World 等で登壇
  - https://www.linkedin.com/in/ankitjaindce

**現チームメンバー** (公式サイトより):
- Ofer Goldstein (Software Engineer)
- Ayushi Gupta (Software Engineer)
- Simrandeep Singh (Software Engineer)
- Davi Maciel Dias (Software Engineer)
- David Bonilla (Growth Marketing Manager)
- Antonija Bilic Arar (Developer Content Lead)

**チーム規模**: YC公式では6名、GetLatkaでは47名 (2024年) と大きく乖離。正確な数値は不明。

## 資金調達

| ラウンド | 時期 | 金額 | 主要投資家 |
|----------|------|------|------------|
| Pre-Seed | 2022年5月 | $2.3M | Y Combinator, Elad Gil, AirAngels, Xoogler Ventures |

- その他の投資家 (20名以上): Shrug VC, Liquid2, GFC, Retool創業者, Applied Intuition創業者, Instabug創業者
  - https://www.aviator.co/news/announcing-our-2-3-million-seed-round
- 累計調達額: $2.42M (PitchBook調べ)
  - https://www.crunchbase.com/organization/aviator-c798
- バリュエーション: 非公開
- Series A以降の調達: 2026年3月時点で公開情報なし
- GetLatkaによると2024年時点で売上$7.1M・従業員47名 (データ精度は検証不可)
  - https://getlatka.com/companies/aviator.co

## プロダクトの変遷

| 時期 | プロダクト/機能 | 概要 |
|------|-----------------|------|
| 2021年 | **MergeQueue** | 最初のプロダクト。バッチ型フォールトトレラントマージキュー自動化 |
| 2022年頃 | **av CLI** (OSS) | スタックドPR管理CLI。Go言語、MIT |
| 2022年頃 | **Stacked PRs** | MergeQueueとav CLIの統合 |
| 2023年5月 | **TestDeck** (Beta) | フレーキーテスト自動検出・隔離 |
| 2024年頃 | **Pilot** | カスタムワークフロー自動化アクション |
| 2025年4月 | **FlexReview** GA | コードレビュー最適化 (SLO管理、AttentionSet、ドメイン専門性スコアリング) |
| 2025年4月 | **Aviator Agents** (Beta) | LLMベースの大規模コードマイグレーション |
| 2025年10月 | **Runbooks** | スペック駆動AI開発。Claude Codeエージェント統合 |
| 2026年頃 | **Releases** | デプロイ管理、ロールバック、チェリーピック、チェンジログ |

- https://www.aviator.co/changelog
- https://www.aviator.co/blog/announcing-testdeck/
- https://www.aviator.co/blog/aviator-runbooks-turn-ai-coding-multiplayer-with-spec-driven-development/

**ピボットの有無**: 明確なピボットなし。MergeQueueを核として開発者ワークフロー全体をカバーするプラットフォーム化を段階的に推進。2025年以降はAIエージェント方向への大きなシフト。

## ユーザーインターフェイス

- `av` CLI (Go製OSS) がエントリポイント
  - `av branch` でスタックPR用の子ブランチ作成
  - `av pr` でPR作成 (親子関係を自動認識)
  - `av sync --all` で親PR更新時に子を自動リベース
  - `av tree` でPR依存グラフを可視化
  - `av switch` でスタック間ナビゲーション
  - `av split-commit`, `av commit --amend` でコミット操作 + 子の自動リベース
- SaaS側ダッシュボードでMergeQueue, FlexReview等を操作
- Chrome拡張でGitHub上にMergeQueue状態を表示
- `aviator-config.yaml` によるコードベース設定
- AI agent向けプラグイン
  - https://github.com/aviator-co/agent-plugins

## 要素技術

### スタックPR管理 (av CLI)
- ブランチメタデータをローカルに保持 (`.git/av/av.db`)
- git merge-baseロジックによるインテリジェントリベース
  - raw `git rebase` では正しく扱えないスタック間のmerge baseを独自計算
- GitHub CLIの認証を利用、GitHub APIでPR操作
- 言語: Go (99.8%)、ライセンス: MIT
- リリース数: 63 (最新 v0.1.16, 2026年2月9日)
- Stars: 468, Forks: 37, Open Issues: 61

### MergeQueue
- バッチ型フォールトトレラントマージオーケストレーション
- 100+同時マージ対応 — 大規模モノレポ向け
- ファストフォワーディング、Affected Targets (モノレポ対応)、Change Sets (クロスリポ)
- フレーキーテスト検出・管理統合
- mainブランチを常にgreenに保つ
  - https://www.aviator.co/merge-queue

### FlexReview
- 静的なCODEOWNERSを動的なコードレビュー割り当てで置換
- SLO管理、AttentionSet (レビュー待ち通知)、ドメイン専門性スコアリング
- Chrome拡張でファイル所有権表示
  - https://www.aviator.co/flexreview

### Releases
- デプロイ、ロールバック、チェリーピック、changelog管理

### Runbooks (2025年10月〜)
- Claude Codeエージェントをサンドボックス環境で実行
- スペック駆動: 自然言語で仕様を記述 → AIエージェントがステップごとに実行
- 各ステップがスタックドPRを生成し、人間がGitHub上でレビュー
- ナレッジグラフ: リポジトリ状態、PRフィードバック、コード変更を自動蓄積
- テンプレートライブラリをOSSで公開
  - https://github.com/aviator-co/runbooks-library

### CI連携
- Buildkite, CircleCI, GitHub Actions をサポート
- API: REST API + GraphQL API + Webhooks

## 料金体系

| プロダクト | Free | Pro | Enterprise |
|-----------|------|-----|------------|
| **MergeQueue** | 15ユーザーまで無料 (全機能) | $12/ユーザー/月 | カスタム (セルフホスト、SAML等) |
| **Releases** | 5ユーザーまで無料 | $8/ユーザー/月 | カスタム |
| **Runbooks** | 月20,000クレジット、月10タスク、1リポ | $10/月〜 (クレジット従量制) | カスタム |
| **Stacked PRs CLI** | 完全無料 (OSS) | — | — |
| **FlexReview** | カスタム | カスタム | カスタム |

- https://www.aviator.co/pricing
- クレジットカード不要、14日間トライアルあり
- 未使用クレジットは翌月繰越
- エンタープライズ: セルフホスト対応、SAML/SCIM

## 会社情報

| 項目 | 詳細 |
|------|------|
| 設立 | 2021年4月 |
| 本社 | サンフランシスコ, CA |
| 認証 | SOC 2 Type II取得済 |
| YCバッチ | S21 |
| 累計調達 | $2.42M |
| 推定売上 | $7.1M (2024年、GetLatka、要検証) |

## 競合環境 (Aviator自身の認識)

Aviatorが自社サイトで明示的に比較ページを設けている競合:

| 競合 | Aviatorの主張する優位性 |
|------|------------------------|
| **Graphite** | エンタープライズ級マージキュー (バッチング、マルチキュー、フレーキーテスト管理) で優位。Graphiteはスタックドpr管理+基本レビュー |
| **Mergify** | ファストフォワーディング、Affected Targets (モノレポ対応)、Change Sets (クロスリポ)、CI最適化、スタック対応マージで優位。Mergifyは設定-as-codeに強み |
| **GitHub Merge Queue** | より高度なカスタマイズ性、並列モード、フレーキーテスト対応で差別化 |

- https://www.aviator.co/aviator-vs-graphite
- https://www.aviator.co/aviator-mergequeue-mergify
- https://www.aviator.co/aviator-github-mergequeue

その他の暗黙的競合: git-branchless, Trunk Merge, Kodiak (現GitKraken)

## OSS戦略

- av CLIをMIT OSSとして提供 → スタックドPRワークフローへの入り口
- CLIユーザーがMergeQueue (有償SaaS) と組み合わせて使うPLG (Product-Led Growth) 戦略
- GitHub Organization (aviator-co): 18リポジトリ
  - 主要OSS: av, agent-plugins, runbooks-library, niche-git, pytest-aviator, gitprotocolio
- Discordサーバーでコミュニティ運営

## AI戦略 (2025年〜)

### 方向性
- 「コンダクター」(単一エージェントとの対話) → 「オーケストレーター」(複数エージェントの並列管理) への移行を予測
  - https://www.aviator.co/blog/the-rise-of-coding-agent-orchestrators/
- 「エンジニアがコードを書く」→ 「エンジニアが仕様を書き、AIが実装する」パラダイムシフトを推進

### Aviator Agents (2025年4月〜 Beta)
- 大規模コードマイグレーション向け
- 対応モデル: OpenAI o1, Claude Sonnet 3.5, DeepSeek R1, Llama 3.1
- GitHubとのエンドツーエンド統合

### agent-plugins
- https://github.com/aviator-co/agent-plugins (MIT, Stars: 4)
- **av-cli プラグイン**: `.git/av/av.db` を検出して自動的にav CLIをエージェントに教示
- **aviator プラグイン**: MCP (Model Context Protocol) サーバー経由でClaude CodeからRunbooksを作成・操作。OAuth認証を自動処理

### 最近の発信
- 「The Rise of Coding Agent Orchestrators」(ブログ)
- 「What Do You Mean by Multiplayer AI Coding?」(ブログ)
- 「Stacked PRs: Code Changes as Narrative」(ブログ)
- ポッドキャスト: Annie Vella (Westpac) を招き「From Software Engineers to Agent Managers」を配信
  - https://www.aviator.co/podcast/from-software-engineers-to-agent-managers

### カンファレンス
- DeveloperWeek 2024: Ankit Jainがモノレポについて登壇
- Monorepo World 2024: マージキューについて発表

## 顧客・事例

- 「1000+ developer teams from startups to Fortune 500s」と公称
- **公開顧客**: Slack (Salesforce), Square (Block), Figma, DoorDash, Benchling, Bosch, Lightspeed
- Color: MergeQueueでセマンティックマージコンフリクト回避の事例
- ターゲット: モノレポを運用する中〜大規模エンジニアリングチーム
- ROI訴求: 「engineers save up to 10 hours a week」

## 脅威モデルへのマッピング

- Aviator自体はセキュリティ検証ツールではなく**プロセス自動化・ガバナンスプラットフォーム**
- MergeQueueによるmainブランチ保護、FlexReviewによるレビュー強制は間接的にSDLC健全性に寄与
- 検査・検証のロジック自体の正しさ保証は提供していない
- attestationや署名付きprovenanceの生成・収集機能はない

### gh-verifyとの差分
- Aviatorは「プロセスを矯正する」(MergeQueue, FlexReview)、gh-verifyは「プロセスの結果を検査する」(lint的ルール群)
- Aviatorにはルールロジックの形式検証に相当する機能がない
- gh-verifyにはMergeQueue的なプロセス強制機能がない (検査のみ)
- Aviatorは主にGitHub中心、gh-verifyはJira等GitHub外ツールとの連携を設計に内包
- Aviatorは$7.1M売上のスタートアップ、gh-verifyはOSS — コスト構造が根本的に異なる
- AviatorのAI戦略 (Runbooks, agent-plugins) はgh-verifyにはない方向性

## 不明な点

- 社名 "Aviator" の由来
- SaaS側のサーバーサイド技術スタック (言語、DB、インフラ)
- 正確なバリュエーション
- Series A以降の追加調達の有無
- 正確な従業員数 (YC: 6名 vs GetLatka: 47名)
- av CLIの正確なコントリビューター数
- 日本市場への展開状況

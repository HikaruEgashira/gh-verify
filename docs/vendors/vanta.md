# Vanta
https://www.vanta.com/
https://developer.vanta.com/

## 会社の歴史

- 2018年創業。サンフランシスコ
- 創業者: **Christina Cacioppo** (CEO)。元Greylock Partners
- 創業の動機: コンプライアンス監査の手作業をソフトウェアで自動化する
- **Y Combinator Winter 2018 (W18)** バッチ
- 社名の由来: 不明

### チーム
**Christina Cacioppo** (CEO & Founder)
- Union Square Ventures, Greylock Partners でベンチャーキャピタリスト
- コンプライアンス自動化市場のカテゴリクリエーター

### 成長
- コンプライアンス自動化カテゴリのマーケットリーダー
- 8,000+ 顧客 (公称)
- Fortune 500企業からスタートアップまで幅広い顧客層

## 資金調達

| ラウンド | 時期 | 金額 | 主要投資家 |
|----------|------|------|------------|
| Seed | 2018年 | — | Y Combinator |
| Series A | 2020年 | $10M | Sequoia Capital |
| Series B | 2022年 | $110M | Craft Ventures (リード) |
| Series C | 2023年 | $150M | Sequoia Capital (リード) |

- Series C時バリュエーション: $2.45B (ユニコーン)

## プロダクトの変遷

| 時期 | プロダクト/機能 | 概要 |
|------|-----------------|------|
| 2018年 | **SOC 2自動化** | 最初のプロダクト。SOC 2監査準備の自動化 |
| 2019年頃 | **ISO 27001対応** | フレームワーク拡張 |
| 2020年頃 | **HIPAA, PCI DSS対応** | 医療・決済向けコンプライアンス |
| 2021年頃 | **Trust Center** | 外部向けセキュリティポスチャ公開ポータル |
| 2022年頃 | **Vendor Risk Management** | サードパーティリスク管理 |
| 2023年頃 | **Vanta API** | カスタム統合・プログラマティックアクセス |
| 2024年頃 | **Vanta AI** | AI制御マッピング、質問回答自動化 |
| 2025年頃 | **MCP Server** | AIアシスタント (Claude, Cursor等) 向けコンプライアンスデータアクセス |

**ピボットの有無**: 明確なピボットなし。SOC 2自動化を核として、対応フレームワーク数 (35+) とプラットフォーム機能 (Trust Center, TPRM, API) を段階的に拡張。

## ユーザーインターフェイス

- SaaSダッシュボードが主なインターフェイス
  - コントロールのパス/フェイル状態をリアルタイム表示
  - フレームワーク横断のコンプライアンス進捗管理
- Trust Center: ブランド化されたセキュリティポータル (スタンドアロンまたはアドオン)
- REST API (`https://api.vanta.com`): OAuth 2.0認証
- MCP Server: AIアシスタントからのプログラマティックアクセス
  - https://github.com/VantaInc/vanta-mcp-server
- Auditor SDK: TypeScript / Java
  - https://github.com/VantaInc/vanta-auditor-api-sdk-typescript
  - https://github.com/VantaInc/vanta-auditor-api-sdk-java

## 要素技術

### コンプライアンス自動化

- **300+統合**: AWS, GCP, Azure, GitHub, GitLab, Bitbucket, Okta, Jira, Slack等
- **1,200+自動テスト**: 毎時実行。コントロール状態をリアルタイム監視
- **35+コンプライアンスフレームワーク**: SOC 2, ISO 27001, HIPAA, PCI DSS, GDPR, HITRUST, FedRAMP, NIST 800-53, NIST CSF, DORA, SOC 1, ISO 27701, CCPA等
- **クロスフレームワークマッピング**: SOC 2で収集した証跡をISO 27001, HIPAA等に再利用

### 自動テスト実行

- APIベースで接続先システムのリソース (アカウント, デバイス, リポ, 脆弱性等) を収集
- 事前構築テスト + ユーザー定義カスタムテストを毎時実行
- バイナリのパス/フェイル判定

### GitHub統合

対応リソース: GitHub Account, Invitation, Repo, Task, Vulnerability

25+の自動テスト:
- 「アプリケーション変更がレビューされているか」(PRレビュー検証)
- 「管理者に対してもブランチ保護ルールが強制されているか」
- 「P0セキュリティイシューが解決されているか」
- 「インシデント管理タスクが完了しているか」
- 「退職者のオフボーディングタスクが完了しているか」

- https://www.vanta.com/integrations/github

### Trust Center

- ブランド化された外部向けセキュリティポータル
- アクティブなコントロールとコンプライアンス状況をリアルタイム表示
- ゲーテッドドキュメント共有 (アクセス管理付き)
- CRM統合 (Salesforce) で収益影響を追跡
- AI搭載Q&A (訪問者向け)

### Vendor Risk Management (TPRM)

- ベンダーインベントリの一元管理
- IDPからのベンダー自動発見
- ビルトインルーブリックによるリスクスコア割当
- AIによるベンダードキュメント分析・質問回答自動化
- 継続的な侵害/脅威モニタリング

### API

REST API (`https://api.vanta.com`), OAuth 2.0認証

| エンドポイント | レート制限 |
|---------------|-----------|
| OAuth | 5 req/min |
| Integration | 20 req/min |
| Management | 50 req/min |
| Auditor | 250 req/min |

2種類のエンドポイント:
1. **Build Integrations**: リソースデータ (アカウント, デバイス, 脆弱性, カスタムリソース) をVantaに送信
2. **Manage Vanta**: 人事管理, テスト結果クエリ, ドキュメント管理, コントロール/フレームワーク照会

- https://developer.vanta.com/docs/vanta-api-overview

### AI機能

- コントロールマッピングの自動化
- セキュリティ質問回答の自動化
- ベンダードキュメント分析
- MCP Server (パブリックプレビュー): Claude, Cursor等からコンプライアンスデータに直接アクセス
  - https://www.vanta.com/resources/meet-the-vanta-mcp-server

## 料金体系

カスタム見積り (年間契約のみ):

| ティア | 参考価格 | 概要 |
|--------|---------|------|
| Essentials | ~$10,000/年 | 単一フレームワーク |
| Plus | カスタム | 複数フレームワーク、拡張自動化 |
| Professional | カスタム | フルAI、高度ワークフロー |
| Enterprise | ~$30,000+/年 | 無制限フレームワーク、専任サポート |

企業規模別参考価格:
- 1-50名: $12K-$28K/年
- 51-200名: $25K-$55K/年
- 201-500名: $50K-$110K/年
- 500名+: $100K-$250K+/年

中央値: $20,000/年。複数年契約で15-30%ディスカウント。監査費用は別途 ($10K-$50K)。

- https://www.vanta.com/pricing

## 会社情報

| 項目 | 詳細 |
|------|------|
| 設立 | 2018年 |
| 本社 | サンフランシスコ, CA |
| YCバッチ | W18 |
| 累計調達 | 約$270M |
| バリュエーション | $2.45B (Series C時) |
| 顧客数 | 8,000+ (公称) |
| 対応フレームワーク | 35+ |
| 統合数 | 300+ |

## 競合環境 (Vanta自身の認識)

| 競合 | Vantaの差異 |
|------|-------------|
| **Drata** | Drataはより技術的に深い自動化、DevOps/CI/CD統合の深度で優位。VantaはセットアップのSQLAの容易さ、300+統合、クロスフレームワークマッピングで優位 |
| **Secureframe** | Secureframeはホワイトグローブサポート、非技術者向け。Vantaはセルフサービス志向、統合エコシステムの広さで優位 |
| **Sprinto** | Sprintoはコスト効率。VantaはAI機能、Trust Center、MCP統合で差別化 |

## 規制対応

| 対応フレームワーク (代表) | カテゴリ |
|---------------------------|----------|
| SOC 2 Type I/II | セキュリティ監査 |
| ISO 27001 | 情報セキュリティ |
| ISO 27701 | プライバシー |
| HIPAA | 医療データ |
| PCI DSS | 決済セキュリティ |
| GDPR | EU データ保護 |
| HITRUST (e1, i1, r2) | 医療セキュリティ |
| FedRAMP | 米国政府クラウド |
| NIST 800-53 | セキュリティ管理策 |
| NIST CSF | サイバーセキュリティ |
| DORA | EU金融サービス |
| SOC 1 | 財務報告統制 |
| CCPA | カリフォルニアプライバシー |

## 脅威モデルへのマッピング

- Vantaは**コンプライアンス自動化プラットフォーム** — SDLC検証をコンプライアンス統制の一部として実施
- テストはバイナリのパス/フェイル — 「ブランチ保護が有効か」「レビューが行われているか」の確認レベル
- SLSA Provenanceの検証、SBOM分析、サプライチェーンアテステーションの深い検査は行わない
- attestationの暗号署名・保管機能はない (コンプライアンスチェックの結果を保管)

### gh-verifyとの差分

- Vantaは「コンプライアンス統制の充足を確認する」(35+フレームワーク)、gh-verifyは「SDLC健全性を深く検査する」(ルールロジック + 形式検証)
- Vantaは**幅広く浅い**: 300+統合、1,200+テスト、35+フレームワークをカバーするが各テストはバイナリ判定
- gh-verifyは**狭く深い**: GitHub SDLCに特化し、PRファイルパッチの解析、スコープ付き変更検証、ポリシーasコード (Rego) で段階的評価
- Vanta: SaaS ($10K-$250K+/年) / gh-verify: OSS — コスト構造が根本的に異なる
- Vantaは監査人・GRC担当者がプライマリユーザー / gh-verifyは開発者がプライマリユーザー
- **補完的**: gh-verifyの詳細なSDLC検査結果をVanta APIのカスタムリソースとして送信し、Vantaのコンプライアンスワークフローに統合可能
  - Vanta APIのBuild Integrations (カスタムリソース、カスタムテスト) が技術的に実現可能なパス
- Vantaは保管とレポートを自社プラットフォームに集約 / gh-verifyは保管をGitHubに委譲
- VantaのMCP Server戦略はAIエージェント時代への対応だが、検証ロジック自体の正しさ保証は提供しない

## 不明な点

- 正確な従業員数
- 技術スタック (言語、DB、インフラ)
- カスタムテストのロジック定義言語/DSL
- GitHub統合の具体的なAPI呼び出しパターン (REST vs GraphQL)
- 日本市場への展開状況
- 監査パートナーとの連携の技術的詳細

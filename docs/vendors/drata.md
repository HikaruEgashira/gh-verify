# Drata
https://drata.com/
https://developers.drata.com/

## 会社の歴史

- 2020年創業。サンディエゴ, カリフォルニア州
- 創業者: **Adam Markowitz** (CEO), **Daniel Marashlian** (CTO), **Troy Markowitz** (COO)
- 創業の動機: 前職Portfolium (学術ポートフォリオネットワーク) でSOC 2準拠を手作業で行った苦痛から、コンプライアンス自動化プラットフォームを着想
- 社名の由来: 不明

### チーム

| 名前 | 役職 | 経歴 |
|------|------|------|
| **Adam Markowitz** | Co-Founder & CEO | NASAスペースシャトルプログラムの航空宇宙エンジニア。USC航空宇宙工学修士。Portfolium創業→Instructure (2019年買収) |
| **Daniel Marashlian** | Co-Founder & CTO | Portfolium Co-Founder & CTO → Instructure Sr. Dir. of Engineering。TweetPhoto/Plixi、Pelotonics等のスタートアップ経験 |
| **Troy Markowitz** | Co-Founder & COO | Portfolium VP of Partnerships。GTM戦略とオペレーション |
| **Aneal Vallurupalli** | CFO | — |
| **Matt Hilary** | VP, Security & CISO | — |
| **Conor Nolen** | CCO | — |
| **Lauren Lynch** | VP of Sales, North America | — |

- 従業員数: **723名** (2026年3月時点)
- エグゼクティブ: 16名
- 取締役: 8名

### 成長の軌跡

| 時期 | イベント |
|------|---------|
| 2020年 | 創業。SOC 2自動化に特化 |
| 2021年6月 | Series A。フレームワーク拡張開始 |
| 2021年11月 | Series B ($100M)。創業16ヶ月でユニコーン ($1B) |
| 2022年12月 | Series C ($200M)。$2B バリュエーション |
| 2024年4月 | **Harmonize.io買収** — AI/MLベースのID・アクセスライフサイクル管理 |
| 2024年5月 | **oak9買収** — Compliance as Code。IaCセキュリティ |
| 2025年2月 | **SafeBase買収** ($250M) — Trust Center。セキュリティレビュー自動化 |
| 2025年 | ARR $98M (推定)。7,000+顧客 |
| 2026年3月 | Agentic AI機能発表 (Agentic TPRM Assessment等) |

## 資金調達

| ラウンド | 時期 | 金額 | 主要投資家 |
|----------|------|------|------------|
| Seed | 2020年 | $3.2M | Cowboy Ventures (リード), Leaders Fund, SV Angel |
| Series A | 2021年6月 | $25M | GGV Capital, Basis Set Ventures, SVCI |
| Series B | 2021年11月 | $100M | ICONIQ Growth (リード), Alkeon Capital, Salesforce Ventures |
| Series C | 2022年12月 | $200M | ICONIQ Growth & GGV Capital (共同リード), IVP, Cowboy Ventures |

- 累計調達: **$328M**
- Series C時バリュエーション: **$2B** (ユニコーン)
- ARR: $98M (2025年1月推定, Sacra)。$59M (2023年) → $95M (2024年) → $98M (2025年) — 61% YoY成長 (2023→2024)

## プロダクトの変遷

| 時期 | プロダクト/機能 | 概要 |
|------|-----------------|------|
| 2020年 | **SOC 2自動化** | 最初のプロダクト。継続的モニタリングと証拠収集の自動化 |
| 2021年頃 | **マルチフレームワーク対応** | ISO 27001, HIPAA, PCI DSS, GDPR等に拡張 |
| 2022年頃 | **Trust Center** | 外部向けセキュリティポスチャ公開ポータル |
| 2023年頃 | **Open API** | REST API。プログラマティックアクセス・カスタム統合 |
| 2024年4月 | **Harmonize.io統合** | AI/ML駆動のID・アクセスライフサイクル自動化 |
| 2024年5月 | **Compliance as Code** (oak9買収) | IaCスキャン、CI/CDパイプライン統合、シフトレフト |
| 2024年頃 | **Third-Party Risk Management** | ベンダーリスク管理、インベントリ、自動評価 |
| 2025年2月 | **SafeBase統合** ($250M買収) | Trust Center強化。1,000+組織の実績統合 |
| 2025年Q2 | **CIS 8.1 / UK Cyber Essentials 3.2** | フレームワーク追加。AWS 45+サービス対応 |
| 2026年3月 | **Agentic AI** | Agentic TPRM Assessment, Agentic Questionnaire Response, AI Trust Center Setup |
| 2026年頃 | **MCP Server** (Beta) | Claude, Cursor等のAIエージェントからコンプライアンスデータにアクセス |

**ピボットの有無**: 明確なピボットなし。SOC 2自動化を核として、買収戦略 (oak9→Compliance as Code, SafeBase→Trust Center) でプラットフォームを拡張。「Trust Management Platform」へのリポジショニングを推進。

## ユーザーインターフェイス

- SaaSダッシュボードが主なインターフェイス
  - コントロールのパス/フェイル状態をリアルタイム/日次で表示
  - フレームワーク横断のコンプライアンス進捗管理
  - リスクスコアリングとレメディエーション追跡
- Trust Center (SafeBase統合): ブランド化された外部向けセキュリティポータル
  - セキュリティレビュー自動化、ゲーテッドドキュメント共有
- REST API v2 (`https://developers.drata.com/`): OAuth 2.1認証
- MCP Server (Beta): AIエージェントからのプログラマティックアクセス
  - OAuth 2.1 + SSO、ユーザーレベル権限、監査ログ
  - Drataホスト型 (マネージド)
- GitHub Marketplace App: `drata-version-control`
  - https://github.com/marketplace/drata-version-control
- GitHub Actions: `drata/compliance-as-code-action`
  - https://github.com/drata/compliance-as-code-action

## 要素技術

### コンプライアンス自動化

- **200+統合**: AWS (45+サービス), GCP, Azure, GitHub, GitLab, Bitbucket, Okta, Jira, Slack等
- **自動テスト**: コントロール状態を日次で監視 (Vantaは毎時)
- **20+コンプライアンスフレームワーク**: SOC 2, ISO 27001:2013/2022, ISO 42001:2023, HIPAA, PCI DSS, GDPR, CCPA, DORA, CMMC, NIST 800-53, NIST CSF, NIST AI, NIST 800-171, FFIEC, SOX ITGC/COBIT, ISO 27701, ISO 27017, ISO 27018, Cyber Essentials, Microsoft SSPA, HITRUST, CCM, CIS 8.1, Custom Frameworks
- **クロスフレームワークマッピング**: 1つのフレームワークで収集した証跡を他のフレームワークに再利用

### GitHub統合

GitHub App (Read-Only) による継続的モニタリング。

**必要な権限**:
- Repository: Administration, Code scanning alerts, Dependabot alerts, Metadata
- Organization: Members
- Account: Email addresses

**マッピングされるテスト** (6テスト):
| テスト | 概要 |
|--------|------|
| Test 6 | 認可された従業員のみがバージョン管理にアクセス |
| Test 7 | 認可された従業員のみがコードを変更 |
| Test 8 | 正式なコードレビュープロセスの存在 |
| Test 9 | 本番コード変更の制限 |
| Test 87 | バージョン管理システムのMFA |
| Test 94 | バージョン管理アカウントの適切な削除 |

**技術的実装**:
- 夜間同期 (Nightly sync)。メールベース+ファジーマッチングでGitHubユーザーをDrata人員にマッピング
- リポジトリ単位でモニタリングスコープを設定可能
- Organization-levelのGitHub App権限が必要 (個人アカウントは不可)

### Compliance as Code (oak9買収)

- IaCスキャン: AWS, Azure, GCP等のクラウドインフラストラクチャコードを解析
- **30+テスト**: ミスコンフィグレーションをスキャンし、コンプライアンス・セキュリティへの影響を検出
- **自動PR生成**: 問題検出時にコントロールコンテキスト、問題箇所、推奨修正を含むPRを自動作成
- CI/CDパイプライン統合: GitHub Actions (`drata/compliance-as-code-action`)、Bitbucket等
- コード→本番環境のデプロイ前後で継続的コンプライアンスモニタリング
- 対応規格: SOC 2, PCI DSS, ISO 27001, GDPR, CCPA, HIPAA, HI TECH, State Regulations, NIST SP 800-53

### Trust Center (SafeBase買収)

- ブランド化された外部向けセキュリティポータル
- **セキュリティレビュー自動化**: AI駆動のアクセスリクエスト管理、ドキュメント共有
- 1,000+組織が利用 (SafeBase時代)。OpenAI, Twilio, CrowdStrike, HubSpot, LinkedIn, T-Mobile等
- セキュリティ関連収益 $15B の促進実績 (SafeBase公称)
- SafeBase Trust API: プログラマティックなトラストセンター管理

### Third-Party Risk Management (TPRM)

- ベンダーインベントリの一元管理
- プロキュアメント/CLMシステムからのベンダー自動同期
- 統合経由のアプリ自動発見
- **Agentic TPRM Assessment** (2026年): AIエージェントがベンダードキュメントを収集、評価基準に照合、ギャップを検出、フォローアップ質問を自動化
- Live Trust Center Integration: ベンダーのDrata Trust Centerから最新の信頼性アーティファクトを自動取得

### API

REST API v2 (`https://developers.drata.com/`)

- OAuth 2.1認証
- エンドポイント単位でRead/Write権限をスコープ設定可能
- APIキー単位でのアクセス制御・失効管理
- インタラクティブな例、複数言語のコードサンプル

主な連携先自動化ツール: Tines, Torq, Tray.io (追加の統合をアンロック)

### AI機能 (Agentic Trust Management)

- **Agentic TPRM Assessment**: ベンダー評価の自動化。ドキュメント収集→基準照合→ギャップ検出→フォローアップ
- **Agentic Questionnaire Response** (Beta): 質問回答のライフサイクル自動化 (コラボレーション、リマインダー、レビュー準備、最終配信)
- **AI Trust Center Setup**: 既存アーティファクトを取り込み、数分でTrust Centerプレビューを生成
- **AI-Generated Criteria**: 構造化された評価基準のAI生成
- **MCP Server** (Beta): Claude, Cursor等のAIエージェントからコンプライアンスデータに直接アクセス
  - OAuth 2.1 + SSO、ユーザーレベル権限、完全監査ログ
  - Drataホスト型 (サーバーレス)
  - ポリシー、コントロール、テスト、リスクのリアルタイムクエリ
- AWS Bedrock上に構築されたAIエンジン

## 料金体系

カスタム見積り:

| ティア | 参考価格 | 概要 |
|--------|---------|------|
| Foundation | ~$7,000-$7,500/年 | 単一フレームワーク。スタートアップ向け |
| Advanced | ~$15,000/年 | 複数フレームワーク、高度な統合、リスク機能 |
| Enterprise | ~$25,000-$50,000+/年 | 高度なGRC、Trust Center、TPRM Pro、監査サポート |

- 中央値 (Vendr): **$25,000/年** ($10,250-$42,750のレンジ)
- 大企業: $75,000-$100,000+/年
- 追加フレームワーク: $3,000-$10,000/フレームワーク/年
- 実装費: 最大$25,000
- Hidden costs (実装、追加フレームワーク、年次更新) で合計20-35%増の可能性

## 買収戦略

| 時期 | 対象 | 金額 | 獲得した機能 |
|------|------|------|-------------|
| 2024年4月 | **Harmonize.io** | 非公開 | AI/ML駆動のID・アクセスライフサイクル管理 |
| 2024年5月 | **oak9** | 非公開 | Compliance as Code、IaCセキュリティスキャン |
| 2025年2月 | **SafeBase** | **$250M** | Trust Center、セキュリティレビュー自動化 |

- 積極的なM&A戦略で「Agentic Trust Management Platform」へのリポジショニングを推進
- SafeBase買収額 ($250M) はDrata自身のSeries C調達額 ($200M) を上回る

## 会社情報

| 項目 | 詳細 |
|------|------|
| 設立 | 2020年 |
| 本社 | サンディエゴ, CA |
| 累計調達 | $328M |
| バリュエーション | $2B (Series C時, 2022年12月) |
| ARR | ~$98M (2025年1月推定) |
| 顧客数 | 7,000+ |
| 従業員数 | 723名 (2026年3月) |
| 対応フレームワーク | 20+ |
| 統合数 | 200+ |
| G2評価 | 4.8/5 |
| AWS | Security Competency Partner, Marketplace提供 |

## 競合環境

| 競合 | Drataの差異 |
|------|-------------|
| **Vanta** | Vantaは400+統合 (Drataは200+)、毎時テスト (Drataは日次)、セットアップの容易さで優位。DrataはCompliance as Code (oak9)、より深い自動化チェック、カスタマイズ性で優位。G2スコアはDrata (4.8) > Vanta (4.6) |
| **Secureframe** | Secureframeはホワイトグローブサポート、非技術者向け。Drataはプラットフォームの深さ、買収による機能統合で優位 |
| **Sprinto** | Sprintoはコスト効率。DrataはAgentic AI機能、Trust Center (SafeBase)、Compliance as Codeで差別化 |
| **AuditBoard (Optro)** | エンタープライズGRC。Drataはクラウドネイティブ・開発者フレンドリーなアプローチ |

### ポジショニング

- Vantaが「プラグ&プレイ」ならDrataは「成長に合わせて拡張できるプラットフォーム」
- Drataは技術的に深いカスタマイズと買収によるフルスタック化で差別化
- Vantaはスタートアップ (小規模企業58%) 寄り、Drataはスケーリング企業 (中規模以上53%) 寄り

## 規制対応

| 対応フレームワーク (代表) | カテゴリ |
|---------------------------|----------|
| SOC 2 | セキュリティ監査 |
| ISO 27001:2013 / 2022 | 情報セキュリティ |
| ISO 42001:2023 | AI管理システム |
| ISO 27701 / 27017 / 27018 | プライバシー・クラウド |
| HIPAA | 医療データ |
| PCI DSS | 決済セキュリティ |
| GDPR | EU データ保護 |
| CCPA | カリフォルニアプライバシー |
| DORA | EU金融サービス |
| CMMC | 国防総省 |
| NIST 800-53 / CSF / 800-171 | サイバーセキュリティ |
| NIST AI | AI リスク管理 |
| FFIEC | 金融 |
| SOX ITGC / COBIT | 財務報告統制 |
| HITRUST | 医療セキュリティ |
| CCM | クラウドセキュリティ |
| CIS 8.1 | セキュリティベンチマーク |
| Cyber Essentials 3.2 | 英国サイバーセキュリティ |
| Microsoft SSPA | マイクロソフトサプライヤー |
| Custom Frameworks | カスタム |

## 脅威モデルへのマッピング

- Drataは**コンプライアンス自動化プラットフォーム** — SDLC検証をコンプライアンス統制の一部として実施
- GitHub統合は6テスト (アクセス制御、コードレビュー、変更管理、MFA、アカウント削除) をカバー
- Compliance as Code (oak9) でIaCスキャンとCI/CDパイプライン統合を提供 — シフトレフトの深さはVantaより優位
- 自動PR生成による修正提案機能あり
- SLSA Provenanceの検証、サプライチェーンアテステーションの暗号署名検証は行わない
- attestationの暗号署名・保管機能はない (コンプライアンスチェックの結果を保管)

### gh-verifyとの差分

- Drataは「コンプライアンス統制の充足を確認する」(20+フレームワーク)、gh-verifyは「SDLC健全性を深く検査する」(ルールロジック + 形式検証)
- Drataは**幅広く浅い**: 200+統合、20+フレームワークをカバーするが各テストはバイナリ判定 (GitHub統合は6テスト)
- gh-verifyは**狭く深い**: GitHub SDLCに特化し、PRファイルパッチの解析、スコープ付き変更検証、ポリシーasコード (Rego) で段階的評価
- Drata: SaaS ($7K-$100K+/年) / gh-verify: OSS — コスト構造が根本的に異なる
- DrataはGRC担当者・セキュリティチームがプライマリユーザー / gh-verifyは開発者がプライマリユーザー
- **Compliance as Code (oak9) の差異**: DrataのCompliance as Codeは主にIaCミスコンフィグレーション検出 (AWS/Azure/GCP) に焦点。gh-verifyはGitHub SDLC/SLSAの健全性検証に焦点。レイヤーが異なる
- **補完的**: gh-verifyの詳細なSDLC検査結果をDrata APIのカスタム統合として送信し、Drataのコンプライアンスワークフローに統合可能
  - Drata Open API v2のカスタム統合が技術的に実現可能なパス
  - gh-verifyのSARIF出力をDrataのCompliance as Codeワークフローに連携
- Drataは保管とレポートを自社プラットフォームに集約 / gh-verifyは保管をGitHubに委譲
- DrataのMCP Server戦略はAIエージェント時代への対応だが、検証ロジック自体の正しさ保証は提供しない
- DrataのAgentic AI (TPRM Assessment等) はベンダー評価・質問回答のワークフロー自動化であり、SDLC検証の自動化ではない

## 不明な点

- 正確な技術スタック (言語、DB、インフラ — AWS Bedrockの使用は確認済み)
- GitHub統合の夜間同期の正確なタイミングとAPI呼び出しパターン (REST vs GraphQL)
- Compliance as Code (oak9) の具体的なテストロジック定義言語/DSL
- Open API v2のレート制限
- 日本市場への展開状況
- SafeBase買収後のプロダクト統合の進捗
- Harmonize.io統合の詳細と現在の機能状況
- 監査パートナー (Schellman, A-LIGN等) との技術的連携の詳細
- IPO計画の有無
- Compliance as CodeのGitHub以外のVCS対応の深さ (GitLab, Bitbucket)

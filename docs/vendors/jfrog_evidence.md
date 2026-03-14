# JFrog Evidence
https://jfrog.com/ja/evidence/
https://jfrog.com/blog/evidence-collection-with-jfrog/

## JFrog社の歴史

| 時期 | イベント |
|------|---------|
| 2008年9月 | 創業。イスラエル・ネタニヤ |
| 2008年 | Artifactory開発開始 |
| 2010年 | 初の大手企業顧客を獲得 |
| 2011年 | Artifactory Enterprise版リリース。商用オープンコアモデル確立 |
| 2012年6月 | Series A ($3.5M) |
| 2014年7月 | Series B (金額非公開)。CloudMunch買収 → JFrog Pipelines基盤技術 |
| 2016年1月 | Series C (金額非公開)。JFrog Xray リリース |
| 2018年10月 | Series D ($165M)。ユニコーン化 (評価額$1B超) |
| 2020年9月16日 | **NASDAQ上場** (FROG)。IPO価格$44/株、調達額約$352M |
| 2021年 | Vdoo買収 ($300M)。IoT/デバイスセキュリティ → JFrog Advanced Security |
| 2023年8月 | Release Lifecycle Management (RLM) GA |
| 2024年6月 | Qwak買収 ($230M)。MLOps → JFrog ML |
| 2025年1月 | **JFrog Evidence Collection** 発表 |
| 2025年9月 | swampUP 2025: **AppTrust**, Evidence Partner Ecosystem, JFrog Fly, AI Catalog 発表 |
| 2026年5月 | JFrog Pipelines EOL予定 |

### 創業者

- **Shlomi Ben Haim** (CEO): 営業・ビジネス畑
- **Yoav Landman** (CTO): Artifactoryの原作者
- **Fred Simon** (Chief Architect): 技術アーキテクチャ

### 社名の由来
- 「J」= Java、「Frog」= ソフトウェアの「飛躍 (leap)」を象徴
- ソフトウェアデリバリーにおける大きな跳躍を支援するという意味

### 創業の背景
- バイナリ管理という普遍的なDevOpsの課題を解決するために創業
- 創業者3名が個人資金でブートストラップし、最初のプロダクトArtifactoryを開発
  - https://portersfiveforce.com/blogs/brief-history/jfrog

## 資金調達

| ラウンド | 時期 | 金額 | 主要投資家 |
|----------|------|------|------------|
| Series A | 2012年6月 | $3.5M | Gemini Israel Ventures |
| Series B | 2014年7月 | 非公開 | VMware |
| Series C | 2016年1月 | 非公開 | Scale Venture Partners, Vintage Investment Partners, Battery Ventures, Sapphire Ventures |
| Series D | 2018年10月 | **$165M** | Insight Partners (リード), Spark Capital, Geodesic Capital, Battery Ventures, Sapphire Ventures, Scale VP, Dell Technologies Capital, Vintage |
| IPO | 2020年9月 | **約$352M** | 公開市場 |

- 累計プライベート調達額: 約$227M (7ラウンド、17投資家)
  - https://www.crunchbase.com/organization/jfrog-ltd
- IPO時: 公開価格$44/株 (当初レンジ$33-37 → $39-41 → 最終$44に引上げ)
  - https://www.globenewswire.com/news-release/2020/09/16/2094260/0/en/JFrog-Announces-Pricing-of-Its-Initial-Public-Offering.html
- IPO時バリュエーション: 約$3.9B (初日終了時は$5.7B超)
  - https://www.nasdaq.com/articles/jfrog-ipo-surges-47-on-its-first-day-of-trading-2020-09-17
- Series D時にユニコーン化
  - https://techcrunch.com/2018/10/04/jfrog-lands-165-m-investment-as-valuation-jumps-over-1-billion/

## プロダクトの変遷

| 時期 | プロダクト | 概要 |
|------|-----------|------|
| 2008年 | **Artifactory** | ユニバーサルアーティファクトリポジトリ。30+パッケージ形式対応 |
| 2011年 | Artifactory Enterprise | 商用版。オープンコアモデル |
| 2014年 | CloudMunch買収 | → JFrog Pipelines (CI/CD) の基盤 |
| 2016年 | **JFrog Xray** | 業界初のユニバーサルSCA。バイナリの再帰的脆弱性・ライセンススキャン |
| 2016年頃 | **JFrog Distribution** | Release Bundleによるソフトウェア配布 |
| 2018年頃 | **JFrog Platform** | Artifactory + Xray + Pipelines + Distribution の統合プラットフォーム |
| 2021年 | Vdoo買収 ($300M) | → JFrog Advanced Security (IoT/デバイスセキュリティ) |
| 2023年8月 | **Release Lifecycle Management (RLM)** | 署名付きイミュータブルなリリース候補管理 |
| 2024年6月 | Qwak買収 ($230M) | → JFrog ML (MLOps、AIモデル管理) |
| 2025年1月 | **Evidence Collection** | 署名付きアテステーション収集 |
| 2025年9月 | **AppTrust** | Evidence + Lifecycle Policies + Application の統合ガバナンス |
| 2025年9月 | **JFrog Fly** | AIエージェント時代のデベロッパープラットフォーム、MCP統合 |
| 2025年9月 | **JFrog AI Catalog** | AIモデルのカタログ・承認管理 |

## ユーザーインターフェイス

- JFrog Platform (Artifactory + Xray + Distribution) の一機能として提供
- **JFrog CLI** から証跡収集を実行:
  ```bash
  jf evd create --predicate=<file> --predicate-type=<type> --repo-path=<path> --key=<private-key> --key-id=<key-id> --markdown=<description>
  ```
- REST APIも利用可能
- Web UIで証跡の閲覧・エクスポート・クエリ
- **Evidence Graph** ビュー: アーティファクト、サブジェクト、関係性の視覚的表示
- Quality Gatesをパイプライン内にインラインで設定

## 要素技術

### 署名付きAttestation収集
- SDLC全体 (ソーススキャン, ビルド, テスト, デプロイ) から暗号署名付きattestationを収集
- attestationを成果物 (artifact) と直接紐づけて保管 — **"system of record"**
  - 証跡が別のコンプライアンスサイロに分離しない設計

### アテステーション形式

**in-toto Statement** 準拠 (SLSA フレームワーク互換):
- **Subject**: アーティファクトのSHA-256チェックサムとArtifactoryパス
- **Predicate**: アテステーション内容 (predicateファイルから読み込み)
- **Predicate Type**: スキーマ識別URI

対応Predicate Type:
- `https://in-toto.io/attestation/link/v0.3`
- `https://in-toto.io/attestation/scai/attribute-report`
- `https://in-toto.io/attestation/runtime-trace/v0.1`
- `https://in-toto.io/attestation/test-result/v0.1`
- `https://in-toto.io/attestation/vulns`
- カスタムタイプ (例: `https://jfrog.com/evidence/integration-test/v1`)

### 署名方式
- **DSSE (Dead Simple Signing Envelope)** 形式でラップ
- 対応署名アルゴリズム: **ECDSA**, **RSA**, **ED25519**
- 対応鍵形式: **PEM**, **SSH**
- 秘密鍵でin-toto Statementに署名 → DSSEエンベロープに格納 → 公開鍵で検証
- OSSツール: DSSE Attestation Online Decoder
  - https://github.com/jfrog/evidence-extractor

### Subject Types

| タイプ | 対象 | 用途 |
|--------|------|------|
| GitHub Evidence | リポジトリ/ワークフロー | ソースコード検証 |
| Build Evidence | ビルド名/番号 | ビルドプロセス証明 |
| Package Evidence | パッケージ名/バージョン | パッケージ検証 |
| Release Bundle Evidence | Release Bundle名/バージョン | 配布検証 |
| Custom Evidence | 任意のアーティファクトパス | ユーザー定義 |

### Quality Gates
- Release Lifecycle Management (RLM) との連携: イミュータブルなRelease Candidate (Release Bundle) をライフサイクルステージ間でプロモーション
- 各ステージのentry/exitポイントにEvidenceベースのポリシーゲートを設置
- 必要なアテステーションが揃わない限り次のステージに進めない
- OIDCバインディングによるアーティファクトのトレーサビリティ確保

### AppTrust (2025年9月〜)
- **"DevGovOps"** コンセプトの実現
- Application + Evidence + Lifecycle Policies の3要素で構成
  - https://jfrog.com/blog/jfrog-apptrust-building-a-trusted-software-supply-chain/
  - https://www.businesswire.com/news/home/20250909565850/en/JFrog-Unveils-AppTrust-DevGovOps-Solution-to-Redefine-Software-Release-Governance

### SLSA対応

| SLSAレベル | JFrogの対応機能 |
|------------|----------------|
| Level 1 | Artifactory: ビルドプロビナンスの自動生成・保存。ビルド情報 (SBOM) を全ビルドに保存 |
| Level 2 | Evidence Collection: 署名付きビルドアテステーション (in-toto/DSSE形式) の生成・検証 |
| Level 3 | GitHub/OCI統合: ソースコード由来のアテステーション取り込み、ビルド来歴の暗号学的検証 |
| Level 4 | AppTrust + RLM: イミュータブルなRelease Bundleとポリシーゲートによるエンドツーエンドの信頼チェーン |

- https://jfrog.com/learn/grc/slsa-framework/

### パートナーエコシステム (12パートナー)

| パートナー | 統合内容 |
|------------|----------|
| **Akto** | APIセキュリティテスト結果を脆弱性Evidenceとして提供 |
| **Akuity (Kargo)** | JFrog Evidenceを消費しデプロイメント判断に利用 |
| **CoGuard** | IaCスキャン結果を署名付きIaCセキュリティアテステーションとして提供 |
| **Dagger** | ソフトウェアエンジニアリングワークフローの署名付きOpenTelemetryトレースをEvidenceとして送信 |
| **GitHub** | GitHub Actions ビルドアテステーション・SBOMを自動変換してJFrogに保存 |
| **Gradle** | Develocity Provenance Governor からビルドスキャンデータを署名付きEvidenceとして統合 |
| **NightVision** | 動的セキュリティテスト・API検出の署名付き脆弱性アテステーション |
| **OCI (Oracle Cloud)** | SLSAビルドアテステーションをビルド来歴として自動取り込み |
| **ServiceNow** | 変更リクエスト、承認、脆弱性例外を署名付きEvidenceとして共有 |
| **Shipyard** | エフェメラル環境のライフサイクルアテステーション (タイムスタンプ、所有者情報) |
| **Sonar** | 静的コード分析の署名付きQuality Gate・セキュリティスキャンアテステーション |
| **Troj.ai** | GenAIレッドチーミング結果を署名付きセキュリティEvidenceとして添付 |

- https://jfrog.com/blog/announcing-jfrog-evidence-partner-ecosystem/
- https://investors.jfrog.com/news/news-details/2025/JFrog-Extends-its-System-of-Record-Solution-Empowering-Application-Delivery-Governance-with-Evidence-from-World-Leading-Companies/default.aspx

## 料金体系

| 項目 | 詳細 |
|------|------|
| 価格 | 非公開 |
| モデル | サブスクリプション (Free / Pro / Enterprise / Enterprise+) |
| Evidence機能 | Cloud Enterprise+ 顧客向けに提供開始 (2025年初頭)。セルフホスト版もQ1 2025 |
| Enterprise+比率 | 全売上の54% (2024年Q4) |

## 財務状況

### 売上推移

| 年度 | 売上 | 前年比成長率 |
|------|------|-------------|
| 2018 | $63.5M | — |
| 2019 | $104.7M | +64.8% |
| 2020 | $150.8M | +44.0% |
| 2021 | $206.7M | +37.0% |
| 2022 | $280.0M | +35.5% |
| 2023 | $349.9M | +24.9% |
| 2024 | $428.5M | +22.5% |
| 2025 | $531.8M | +24.1% |

- https://stockanalysis.com/stocks/frog/revenue/

### 主要財務指標 (2026年3月時点)

| 指標 | 値 |
|------|-----|
| 時価総額 | 約$4.9B |
| 企業価値 (EV) | $4.22B |
| 現金 | $704M |
| ネットキャッシュ | $692M |
| GAAPグロスマージン | 77.1% |
| Non-GAAPグロスマージン | 83.8% |
| フリーキャッシュフロー | $107.8M |
| 最終損益 | -$71.8M (GAAP) |
| 従業員数 | 約1,800名 |
| クラウド売上成長率 | +41% |
| ARR $1M超顧客 | 71社 |
| ARR $100K超顧客 | 286社以上 |

- https://investors.jfrog.com/news/news-details/2025/JFrog-Announces-Fourth-Quarter-and-Fiscal-2024-Results/default.aspx
- https://stockanalysis.com/stocks/frog/statistics/

## 会社情報

| 項目 | 詳細 |
|------|------|
| 設立 | 2008年9月 |
| 本社 | サニーベール, CA / ネタニヤ, イスラエル (二拠点) |
| 上場 | NASDAQ: FROG (2020年9月) |
| 従業員数 | 約1,800名 |
| 主要顧客 | Fortune 100の82%が利用 |
| 公開顧客名 | Amazon, Meta, Google, Netflix, Uber, VMware, Spotify |

## 競合環境 (JFrog自身の認識)

| 競合 | カテゴリ | JFrogとの差異 |
|------|----------|---------------|
| **Sonatype Nexus** | アーティファクトリポジトリ | 最大の直接競合。Nexusは30+形式未対応。Sonatype側は脆弱性検出精度で80%優位と主張 |
| **GitHub Packages** | パッケージレジストリ | GitHub統合が強み。npm, Maven, NuGet, Docker, RubyGemsのみ。パッケージ形式の幅でJFrogが優位 |
| **AWS CodeArtifact** | クラウドネイティブリポ | AWS特化。対応形式は限定的 |
| **CloudSmith** | クラウドリポ | 主要形式をカバーするが統合プラットフォームではない |
| **Snyk** | SCA/セキュリティ | セキュリティ特化。JFrogはリポジトリ+セキュリティ統合で差別化 |
| **Checkmarx** | SAST/DAST | セキュリティスキャン特化 |

- Fortune 500企業がSnyk/CheckmarxからJFrogに移行する事例あり
  - https://jfrog.com/blog/why-enterprise-and-fortune-500-companies-are-leaving-snyk-and-checkmarx-for-jfrog/

## 規制対応

| 規格/フレームワーク | 対応状況 |
|---------------------|----------|
| **SOC 2 Type II** | 取得済み。年次で監査・更新 |
| **ISO 27001:2013** | 認証取得済み (情報セキュリティマネジメント) |
| **ISO 27701:2019** | 認証取得済み (データプライバシー) |
| **ISO 27017:2014** | 認証取得済み (クラウドセキュリティ) |
| **NIST SP 800-218 (SSDF)** | コンプライアンスガイドv1.3を公開。プラットフォーム機能が各要件にマッピング |
| **SLSA** | Evidence + RLM でLevel 1-4準拠を支援 |
| **EU Cyber Resilience Act** | AppTrustによるコンプライアンス支援 |
| **Executive Order 14028** | SBOMサポート、ビルドプロビナンス、サプライチェーンセキュリティで対応 |
| **FedRAMP** | 「FedRamp Compliant」との記載あるが、FedRAMP Marketplaceでの正式確認は未実施 |

- https://jfrog.com/trust/certificate-program/
- https://jfrog.com/blog/jfrog-nist-800-218-compliance/

## 最近の動向 (2024〜2026年)

### swampUP 2025 (2025年9月, Napa Valley)
- **AppTrust** 発表: "DevGovOps" コンセプト。Application + Evidence + Lifecycle Policies
- **Evidence Partner Ecosystem** 発表: 12パートナーとのネイティブ統合
- **JFrog Fly**: AIエージェント時代のデベロッパープラットフォーム。MCPサーバー、VS Code/Cursor統合
- **JFrog AI Catalog**: AIモデルのカタログ化・承認管理
- **Agentic Software Supply Chain Security**: MCPサーバー + GitHub統合でAIエージェント開発のセキュリティ
  - https://jfrog.com/blog/jfrog-swampup-2025-live-updates/
  - https://www.forrester.com/blogs/jfrog-swampup-2025-the-agentic-development-era-emerges-from-the-swamp/

### 買収
- 2024年6月: Qwak ($230M) - MLOps/AIモデル管理
  - https://en.globes.co.il/en/article-jfrog-buys-israeli-ai-co-qwak-for-230m-1001482621

### その他
- JFrog Pipelines: 2026年5月1日にEOL予定
- 株価52週レンジ: $27.00 - $70.43 (2026年3月時点で約$41)
- Bank of Americaからエンタープライズテクノロジーイノベーション賞を受賞

## 脅威モデルへのマッピング

- JFrog Evidenceは**証跡の収集・保管・クエリ**に特化
- attestationの暗号署名により「誰が何をいつ行ったか」の改竄防止を保証
- ただし**attestationの内容の正しさ** (ルールロジックが正しいか) は保証しない
  - 署名は「このattestationは改竄されていない」を証明するが「このattestationの判定ロジックが正しい」は証明しない
- 規制フレームワーク (NIST SSDF, SOC 2, ISO 27001等) への対応が幅広い

### gh-verifyとの差分
- JFrog Evidenceは「証跡を集めて保管する」、gh-verifyは「ルールで検査して判定する」
- JFrog: 暗号署名でattestationの完全性を保証 / gh-verify: 形式検証でルールロジックの正しさを保証
- JFrog: 12+パートナーとのネイティブ連携 / gh-verify: Jira等との連携は設計に余地あり・未実装
- JFrog: パイプラインにインライン / gh-verify: オンデマンド or CI
- JFrog: 規制フレームワーク (SOC 2, ISO 27001等) を広くカバー / gh-verify: SLSA概念ベース + SOCレポート生成 (構想段階)
- JFrog Evidenceにはlint的な大量ルール高速実行の概念がない
- gh-verifyは保管をGitHubに委譲し自前で持たない (JFrogはArtifactoryに集約)。gh-verifyの価値はGitHubにできない検証にある
- JFrog: $531.8M売上のNASDAQ上場企業 / gh-verify: OSS — 規模が根本的に異なる
- JFrogのAI戦略 (Fly, AI Catalog, MCP) はエージェント時代のプラットフォーム化。gh-verifyにはこの方向性がない

## 不明な点

- Series B, Series Cの正確な調達額
- JFrog Distributionの正確なリリース年 (2016-2018年頃と推定)
- FedRAMP認証の正式取得有無
- Evidence機能の正確な価格
- Evidence Collection の初期プレビュー (swampUP 2024) 有無

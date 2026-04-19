# Zluri
https://www.zluri.com/
https://www.zluri.com/catalog
https://www.zluri.com/our-story

## 会社の歴史

- 2020年創業。本社はカリフォルニア州ミルピタス (691 S Milpitas Blvd, St 217)、エンジニアリング拠点はインド・ベンガルール
- 創業者: **Ritish Reddy** (CEO), **Sethu Meenakshisundaram** (Co-founder), **Chaithanya Yambari** (Co-founder & CTO)
- 創業の動機: 「忘れられたサブスクリプション」(Forgotten Subscriptions) — SaaS利用が組織内で増殖し、IT部門が把握できない領域 (シャドーIT) と未使用ライセンスのコストが膨らむ問題を解決
- **Y Combinator非参加** (Vanta/Delveとは異なるインドルーツのVCルート)
- 社名の由来: 公式説明なし

### 創業者

3名はいずれも **KNOLSKAPE** (コーポレートラーニング/ゲーミフィケーションSaaS、30か国に展開) の出身。同社で長期間共に働いた経験から、SaaSスプロールの実態を一次体験している。

**Ritish Reddy** (CEO & Co-founder)
- KNOLSKAPE創業チームメンバー、Chief Marketing Officer
- S P Jain School of Global Management で Advanced Corporate Finance and Banking Management の MBA
  - https://www.linkedin.com/in/ritish-reddy-7bb99914/

**Sethu Meenakshisundaram** (Co-founder)
- KNOLSKAPE創業チームメンバー、30か国展開を主導
  - https://www.linkedin.com/in/sethu-ms/

**Chaithanya Yambari** (Co-founder & CTO)
- KNOLSKAPE出身

### 成長

- 2024年時点で **約250社** の顧客、**約$15M ARR** (Latka参照値)
- 中堅企業 (100〜999名規模) を主たる顧客層とする
- 顧客の主張: SaaS支出を平均30%削減
- Series A〜B間でチーム規模をほぼ倍増、米国市場への本格進出 (カリフォルニアオフィス開設)

## 資金調達

| ラウンド | 時期 | 金額 | 主要投資家 |
|----------|------|------|------------|
| Seed | 2021年1月 | $2M | Endiya Partners, Kalaari Capital |
| Series A | 2022年1月 | $10M | **MassMutual Ventures** (リード), Endiya Partners, Kalaari Capital |
| Series B | 2023年7月 | $20M | **Lightspeed Venture Partners** (Dev Khare、リード), MassMutual Ventures, Endiya Partners, Kalaari Capital |

- 累計調達額: 約$32.5M
  - https://www.crunchbase.com/organization/zluri-96e5
- バリュエーション (Series B時): 非公開
- Series B以降の追加ラウンドは2026年4月時点で未発表

## プロダクトの変遷

| 時期 | プロダクト/機能 | 概要 |
|------|-----------------|------|
| 2020年〜 | **SaaS Management Platform (SMP)** | 初期プロダクト。SaaS発見、ライセンス管理、コスト最適化 |
| 2022年頃 | **SaaSOps** | discover/manage/optimize/secure/automate の5機能を統合ダッシュボード化 |
| 2023年 | **マルチプロダクト化** | Series B資金で単一製品からマルチ製品へ。エンタープライズ・ミッドマーケット両対応 |
| 2023〜2024年 | **User Access Reviews** | 規制対応の access certification 自動化 (SOX/SOC 2/HIPAA) |
| 2024年 | **Identity Lifecycle Management** | Joiner/Mover/Leaver 自動化、HRIS連動プロビジョニング |
| 2025年 | **AuthKnox 検出エンジン** | 静的アイデンティティデータと動的利用情報を統合する特許取得済みディスカバリー |
| 2026年3月 | **Zluri Identity Security Platform** | 人間/マシン/AI 全アイデンティティ統合管理。AIエージェント・サービスアカウントを含む非ヒューマンIDのガバナンスへ拡張 |

**ピボットの有無**: 明確なピボットなし。初期の SaaS Management から **Identity Governance and Administration (IGA)** への段階的な再ポジショニング。SaaS スプロール問題からアイデンティティスプロール問題へと射程を拡大。

## ユーザーインターフェイス

- Web SaaS ダッシュボードがメイン
  - SaaS インベントリ、ライセンス使用率、シャドー IT 検出、契約・更新管理を一元表示
  - アクセスレビューワークフロー、JML (Joiner/Mover/Leaver) 自動化エンジン
- ブラウザエージェント: エンドユーザーのSaaS利用情報を補完収集
- AWS Marketplace で提供
  - https://aws.amazon.com/marketplace/pp/prodview-thzrlnakkww5k
- 公開API/SDK は限定的に提供 (主要ユースケースは事前ビルドのコネクタ経由)

## 要素技術

### AuthKnox ディスカバリーエンジン (特許取得済み)

- **静的アイデンティティデータ** (HRIS、SSO、財務システム) と **動的利用情報** (ブラウザエージェント、APIアクティビティ) をマージ
- 競合 (Torii、BetterCloud等) より深いシャドー IT 検出を主張
- 検出ソース: SSO (Okta/Azure AD)、HRIS (Workday/BambooHR)、財務 (QuickBooks/NetSuite)、エンドポイント (MDM)、ブラウザエージェント、直接 API

### Identity Governance and Administration (IGA)

- **Joiner/Mover/Leaver (JML) ワークフロー**: HRイベントトリガで複数SaaSへの自動プロビジョニング/デプロビジョニング
- **Access Reviews / 認証**: 定期的な権限レビュー、レビュアーアサイン (フォールバック付き)、通知、承認/変更/取消アクションの自動化
- **Segregation of Duties (SoD) コントロール**: 職務分離違反の検出
- **Identity Risk Intelligence System**: アイデンティティシグナルの相関分析でリスクを優先度付け

### Identity Security Platform (2026年3月拡張)

- **人間 ID** (従業員、契約者) + **マシン ID** (サービスアカウント) + **AI ID** (AI エージェント、自動化ボット) の統合ガバナンス
- AI エージェント時代の非ヒューマンID爆発に対応
  - https://www.helpnetsecurity.com/2026/03/23/zluri-identity-security-platform-expanded/

### インテグレーション

- **300+ プレビルドコネクタ**: G Suite, Office 365, Salesforce, Slack, Asana, Okta, Workday, GitHub, Jira, Trello, Grammarly, AWS, Azure, GCP 等
- HRIS統合: Workday, BambooHR, Rippling 等
- SSO統合: Okta, Azure AD, Google Workspace
- 財務統合: QuickBooks, NetSuite, Xero
  - https://www.zluri.com/catalog

### GitHub 統合

対応リソース: GitHub Account / Organization Member / Permissions

主要ユースケース:
- **GitHub Access Review**: 組織メンバーの権限定期レビュー (SOC 2/SOX準拠)
- レビュアーアサイン → 通知 → 承認/変更/取消アクション → 監査用エビデンス出力 までを自動化
- インサイトエンジンによるリスクハイライト (例: 過剰権限、長期未使用アカウント)
- 「手動レビューより 10 倍高速」と主張
  - https://www.zluri.com/access-reviews/github-access-review
  - https://www.zluri.com/catalog/github

**GitHub 統合の限界**:
- アクセスレビュー (誰が GitHub にアクセスできるか) のみで、SDLC 健全性 (PRレビュー履行、ブランチ保護、シークレットスキャン、SLSA Provenance、SBOM 等) の検査は行わない
- 公開資料からは GitHub Outside Collaborator/OAuth App/SSH キー/PAT の網羅的検出は確認できない
- code scanning / Dependabot / supply chain attestation 等の SDLC 領域はスコープ外

## 料金体系

- **モデル**: 年間サブスクリプション、見積りベース (公開プライスリストなし)
- **参考価格**: $4〜$8 per user/month (サードパーティ報告値)
- **典型レンジ** (組織規模により): 推定 $20K〜$100K+/年
- 商談・デモ経由のセールスサイクル
  - https://www.zluri.com/

## 顧客・事例

- **約250社** の顧客 (2024年)
- 公開ケーススタディ: エンタープライズ Fintech 企業で SaaS 重複により $3M のコスト削減機会を発見
- ターゲット: ミッドマーケット (100〜999名)、規制業界 (Fintech、ヘルスケア)、SaaS スプロールに苦しむテックカンパニー

## 会社情報

| 項目 | 詳細 |
|------|------|
| 設立 | 2020年 |
| 本社 | Milpitas, CA (US法人) + ベンガルール (インド開発拠点) |
| 累計調達 | 約$32.5M |
| 顧客数 | 約250社 (2024年) |
| ARR | 約$15M (2024年) |
| 統合数 | 300+ |
| 自社認証 | SOC 2 Type II, ISO/IEC 27001, ISO 27701, GDPR, NIST, PCI-DSS |

## 競合環境

| 競合 | カテゴリ | Zluriとの差異 |
|------|----------|---------------|
| **BetterCloud** | SaaS Management (#1 G2 Leader) | エンタープライズ向けライフサイクル管理で先行。価格透明 (per-user)。Zluri は AuthKnox 検出深度で対抗 |
| **Torii** | SaaS Management | ワークフロービルダー、シャドー IT 検出に強み。Zluri はカスタム見積り、Torii は基本プラン透明価格 |
| **Productiv** | SaaS Management | アプリ利用エンゲージメント分析、コンプライアンス at scale で差別化 |
| **Zylo** | SaaS Spend Management | SaaS支出最適化に特化、Zluriはアクセス管理まで包含 |
| **SailPoint / Saviynt** | レガシー IGA | エンタープライズ IGA の老舗。Zluri は「SaaS-first」「実装コスト低減」で差別化 |
| **Okta Identity Governance** | IDaaS拡張IGA | Okta基盤上の IGA。Zluri は SSO 中立 (Okta/Azure AD/Google Workspace 全対応) |
| **Lumos** | アクセスマネジメント | YC W20、Slack/Web経由のセルフサービスアクセス申請に強み |
| **Opal** | 特権アクセス管理 | エンジニアリング向け Just-in-Time アクセス、Zluri はビジネス全体のアクセスガバナンス |

Zluri のポジショニング: 「Next-Gen IGA」「SaaS-first」「Visibility-first」「実装期間短縮」。レガシーIGA (SailPoint等) のコスト・実装オーバーヘッド批判と、SMP (BetterCloud等) のセキュリティ機能不足の中間ポジションを狙う。

## 規制対応

| 対応領域 (顧客の規制対応支援) | カテゴリ |
|-------------------------------|----------|
| SOC 2 Type I/II | アクセスコントロール証跡 (User Access Review) |
| SOX | 職務分離・特権アクセスレビュー |
| HIPAA | アクセスログ・最小権限原則 |
| ISO/IEC 27001 | アクセス管理コントロール |
| GDPR | データアクセス権限管理 |
| PCI-DSS | 特権アカウント管理 |
| NIST AC (Access Control) ファミリ | アクセスポリシー実装 |

注: Zluri 自身が監査人ではない。Vanta/Delve のような **コンプライアンス自動化** ではなく、IGA レイヤから **アクセス統制の証跡** を提供することで監査をサポートする位置づけ。

## 脅威モデルへのマッピング

- Zluri は **Identity Governance & SaaS Management** プラットフォーム — SDLC 検証ツールではない
- アクセス統制 (Who can access what?) のレイヤを担い、**コードの整合性・サプライチェーン整合性** (What is in the artifact, and was it built correctly?) のレイヤは扱わない
- attestation/SLSA Provenance/SBOM/署名検証は範囲外
- GitHub 統合はメンバー権限の certification のみで、PR レビュー履行・ブランチ保護・シークレットスキャン・コードスキャンは検査しない

### gh-verifyとの差分

- Zluri は「**誰がアクセスできるか** を継続的に統制する」(IGA レイヤ)、gh-verify は「**SDLC が健全に運用されているか** を検査する」(コントロール検証レイヤ)
- 検査対象が直交: Zluri = アイデンティティ + SaaS インベントリ / gh-verify = GitHub リポジトリ運用・PR・ブランチ保護・provenance
- Zluri はバイナリのコンプライアンス判定 (アクセスレビュー実施 = ✓) / gh-verify はルールロジック + 形式検証で **検証ロジック自体の正しさ** を保証
- Zluri はコマーシャル SaaS ($20K〜$100K+/年) / gh-verify は OSS — コスト構造が異なる
- Zluri は GRC/IT 部門がプライマリユーザー / gh-verify は開発者がプライマリユーザー
- **補完的**: Zluri が GitHub の **アクセス権限統制** (誰が org に居るか、各人の権限が適切か) を担い、gh-verify が **SDLC 運用統制** (PR がレビューされたか、ブランチ保護が機能したか、provenance が揃っているか) を担う配置が自然
- Vanta との差分: Vanta は 35+ フレームワークで GitHub に 25+ テスト (バイナリ判定) を実施。Zluri は GitHub テストではなく **GitHub アクセス権限のレビュー実施プロセス** を提供。両者の GitHub 統合は粒度が大きく異なる
- Delve との差分: Delve はコンプライアンスレポート生成自体に問題あり (2026年3月スキャンダル)。Zluri は IGA という別カテゴリで、レポート生成ではなく **アクセス統制の継続実行** が本質
- Zluri の AuthKnox 検出 (シャドー IT) は組織が把握していない GitHub Enterprise アカウントや個人アカウントでの社内コード保管リスクの発見に使える可能性あり

### 評価

- Zluri は IGA カテゴリで Lightspeed リードの $32.5M を調達し、2026年に AI/非ヒューマンID対応へ拡張中の成長企業
- gh-verify とは **領域非競合**。むしろ「アクセス統制 (Zluri) × SDLC 検証 (gh-verify) × コンプライアンスレポート (Vanta)」という3層が組み合わさる構図
- Zluri 自体の信頼性: SOC 2 Type II + ISO 27001 自社取得、Lightspeed等のトップティア VC リード、Delve のような偽装疑惑なし

## 不明な点

- 正確な従業員数 (公称されているが検証可能な情報源は限定的)
- Series B 以降のバリュエーション
- AuthKnox の特許番号・技術詳細 (静的×動的データのマージアルゴリズム)
- GitHub 統合で利用される具体的な GitHub API スコープ (REST/GraphQL、必要 OAuth scope)
- AI エージェント識別の検出メカニズム (2026年3月発表) の技術詳細
- 公開 REST/GraphQL API の網羅性 (主要ユースケースは事前ビルドコネクタ経由のため、外部開発者向け API ドキュメントは限定的)
- 日本市場での販売実績
- GitHub Enterprise Server (オンプレ) 対応の有無

# JFrog 直接競合

## 調査メタデータ

- 調査日: 2026-03-15
- 対象会社: JFrog
- 分類: 直接競合

## 対象範囲

この文書では、JFrog Evidence と同様に evidence / attestation / policy-as-code を
主戦場に持つ会社だけを扱う。

## 分析軸

| 軸 | 観点 | 重要度 |
|----|------|--------|
| 証跡モデル | attestation を第一級で扱えるか | 高 |
| 証跡の集約 | SDLC 全体から異種証跡を集められるか | 高 |
| ポリシー実行 | verify, gate, admission control まで持つか | 高 |
| artifact 連携 | registry / package / release と強く結びつくか | 高 |
| 導入自由度 | on-prem, air-gap, 既存基盤との相性 | 中 |

## 競合会社別の整理

### Scribe Security

最も近い直接競合である。証跡を収集し、署名し、検証し、policy-as-code と
proof of compliance までつなげる構成が JFrog Evidence にかなり近い。

**強い点**

- Human と AI generated code の両方を含む observability と attestations を前面に出す
- SBOM-centric software trust center を提供する
- policy-as-code と proof of compliance を同時に扱う

**弱い点**

- Artifactory のような artifact repository を中心にした運用重力は弱い
- 既に JFrog を中核にしている組織では、運用の主語が二重化しやすい

**JFrog とのギャップ**

- Scribe は attestation/compliance-first
- JFrog は artifact/system-of-record-first

### Chainloop

attestation-native な直接競合である。contract-based workflow を持ち、
何を evidence として収集し、どう検証するかを先に定義できる。

**強い点**

- attestation を signed and verifiable unit of data として明確に扱う
- contract-based workflow により、収集物と検証条件を先に固定できる
- on-prem や air-gapped 構成を取りやすい

**弱い点**

- repository / package registry を中心にした企業内標準になりにくい
- JFrog と比べると、広範な artifact distribution 文脈は弱い

**JFrog とのギャップ**

- Chainloop は attestation workflow の純度が高い
- JFrog は artifact, build, release governance を既存基盤へ接続しやすい

### Harness

Harness Software Supply Chain Security Assurance は準直接競合である。
CI/CD と policy enforcement を同じ運用面で扱える点が強い。

**強い点**

- attest and store を公式に掲げる
- SLSA provenance, Cosign, policy enforcement と接続する
- self-managed enterprise や air-gapped の話がしやすい

**弱い点**

- 公式ドキュメント上、SBOM attestation は container image 限定の制約がある
- JFrog のような registry/system-of-record の広がりとは異なる

**JFrog とのギャップ**

- Harness は pipeline-first
- JFrog は artifact-first

## ギャップマトリクス

| 領域 | JFrog | Scribe | Chainloop | Harness |
|------|-------|---------|-----------|---------|
| Evidence 集約 | 非常に強い | 強い | 中 | 中 |
| Attestation の純度 | 強い | 非常に強い | 非常に強い | 中 |
| Artifact 連携 | 非常に強い | 中 | 弱い | 中 |
| Policy / Compliance | 強い | 非常に強い | 強い | 強い |
| SCM/CI との低摩擦 | 中 | 中 | 中 | 中 |
| 導入自由度 | 中 | 中 | 強い | 強い |

## 根本原因分析

### ギャップ1: Scribe / Chainloop の attestation 純度が高い

| 層 | 問い | 回答 |
|----|------|------|
| 表層 | なぜ証跡モデルで差が出るか | Scribe と Chainloop は attestation 自体を主語にしている |
| 中層 | なぜその設計を取るのか | compliance と verification を中心に製品設計しているから |
| 深層 | なぜ JFrog は別方向か | JFrog は artifact system of record から拡張しているから |
| 根本 | 出自の差 | attestation-first と artifact-first の違い |

## 判断指針

- 既に Artifactory を標準基盤にしており、evidence を artifact と強く結びつけたいなら JFrog
- attestation と compliance を主語にゼロベース設計したいなら Scribe または Chainloop
- CI/CD と enforcement を一体で回したいなら Harness

## 参照ソース

- [JFrog Evidence](https://jfrog.com/ja/evidence/)
- [JFrog AppTrust](https://jfrog.com/ja/apptrust/)
- [JFrog Evidence Collection Solution Sheet](https://jfrog.com/solution-sheet/evidence-collection/)
- [JFrog Press Release 2025-09-09](https://jfrog.com/press-room/jfrog-extends-its-system-of-record-solution-empowering-application-delivery-governance-with-evidence-from-world-leading-companies/)
- [Scribe Security](https://scribesecurity.com/scribe-security-4/)
- [Scribe Pricing](https://scribesecurity.com/ja/pricing/)
- [Chainloop Quickstart](https://docs.chainloop.dev/quickstart)
- [Chainloop CLI Reference](https://docs.chainloop.dev/command-line-reference/cli-reference)
- [Harness SSCA Key Concepts](https://developer.harness.io/docs/software-supply-chain-assurance/get-started/key-concepts/)
- [Harness SSCA Support Matrix](https://developer.harness.io/docs/software-supply-chain-assurance/ssca-supported/)

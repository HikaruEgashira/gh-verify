# JFrog 競合分析

## 調査メタデータ

- 調査日: 2026-03-15
- 対象会社: JFrog
- 対象プロダクト: JFrog Evidence
- 参照上の位置づけ: AppTrust 配下の evidence / attestation 基盤として解釈

## 対象会社の現在地

JFrog Evidence は単なる署名保管庫ではない。JFrog の公式説明では、
署名された証拠の検証可能な証跡を保持する「信頼できる記録システム」として
位置づけられている。

- SDLC の各工程で生じる証跡を収集する
- artifact や package と証跡を結びつける
- JFrog CLI, REST API, GraphQL API から扱える
- 2025-09-09 公表の AppTrust 文脈では evidence-based controls を支える下位基盤として機能する

競合調査は責務ごとに次の 2 文書へ分離する。

- [直接競合](./direct-competitors.md)
- [周辺競合](./adjacent-competitors.md)

## 分類方針

- 直接競合:
  evidence / attestation / policy-as-code を主戦場に持ち、JFrog Evidence と機能の中心が重なる会社
- 周辺競合:
  SCM native, cloud deploy gate, SBOM/compliance 製品など、導入判断では比較対象になるが主戦場が完全一致しない会社

## 速い結論

- 主戦場の直接競合は Scribe Security と Chainloop
- 実運用比較で強く出る準直接競合は Harness
- 周辺競合として GitHub, GitLab, Google Cloud, Anchore を分離して保持する

## 参照導線

- 主戦場比較を読む場合:
  [direct-competitors.md](./direct-competitors.md)
- 周辺比較を読む場合:
  [adjacent-competitors.md](./adjacent-competitors.md)

## 参照ソース

- [JFrog Evidence](https://jfrog.com/ja/evidence/)
- [JFrog AppTrust](https://jfrog.com/ja/apptrust/)
- [JFrog Evidence Collection Solution Sheet](https://jfrog.com/solution-sheet/evidence-collection/)
- [JFrog Press Release 2025-09-09](https://jfrog.com/press-room/jfrog-extends-its-system-of-record-solution-empowering-application-delivery-governance-with-evidence-from-world-leading-companies/)

# JFrog 周辺競合

## 調査メタデータ

- 調査日: 2026-03-15
- 対象会社: JFrog
- 分類: 周辺競合

## 対象範囲

この文書では、JFrog Evidence の主戦場と完全一致はしないが、導入判断で比較対象になった
周辺競合を収集する。

## 収集した周辺競合

### GitHub

GitHub Artifact Attestations は部分競合である。GitHub Actions 中心の provenance と
verification を低摩擦で導入できる。

**強い点**

- GitHub 上で attestation を管理できる
- `gh attestation verify` による offline verification を持つ
- admission controller による enforcement の導線がある

**弱い点**

- GitHub 以外の工程から来る異種証跡を統合するハブとしては弱い
- artifact repository 起点の governance 製品ではない

**JFrog との距離**

- GitHub は SCM/CI-native
- JFrog は cross-tool evidence hub

### GitLab

GitLab は release evidence の機能を持つが、現時点では JFrog Evidence の直接代替というより
SCM 一体型の限定機能に近い。

**強い点**

- release evidence により release 時点の snapshot を残せる
- GitLab 内に閉じたチームでは導入が容易

**弱い点**

- release evidence は snapshot 的で、JFrog のような evidence hub と同一ではない
- attestations API は成熟度の面で慎重に見るべきである

**JFrog との距離**

- GitLab は release-native
- JFrog は SDLC 横断の record system

### Google Cloud

Google Cloud は Binary Authorization と Artifact Analysis の組み合わせで、
deploy gate と SBOM 取り込みを提供する準競合である。

**強い点**

- Binary Authorization により deploy-time gate が明確
- Artifact Analysis は SBOM をアップロードして署名付き参照を保持できる

**弱い点**

- GCP 依存が強い
- cross-tool evidence hub というより、GCP 上の deploy trust に寄る

**JFrog との距離**

- Google Cloud は cloud-enforcement-first
- JFrog は artifact-governance-first

### Anchore

Anchore は SBOM と compliance を中心とした隣接競合である。

**強い点**

- SBOM 管理とポリシー運用が強い
- supplier / customer との compliance reporting に相性がよい

**弱い点**

- 証跡ハブ全体ではなく、主軸は SBOM とスキャンの周辺にある
- JFrog Evidence のような evidence aggregation と同一ではない

**JFrog との距離**

- Anchore は SBOM-centric
- JFrog は evidence-centric かつ artifact-linked

## 導入判断での位置づけ

- GitHub:
  SCM と CI に閉じた provenance 比較軸
- GitLab:
  release-native な証跡比較軸
- Google Cloud:
  deploy gate と cloud enforcement の比較軸
- Anchore:
  SBOM と compliance reporting の比較軸

## 参照ソース

- [GitHub Manage Artifact Attestations](https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/manage-attestations)
- [GitHub Verify Attestations Offline](https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/verify-attestations-offline)
- [GitHub Enforce Artifact Attestations](https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/enforce-artifact-attestations)
- [GitLab Release Evidence](https://docs.gitlab.com/user/project/releases/release_evidence/)
- [GitLab Attestations API](https://docs.gitlab.com/api/attestations/)
- [Google Cloud Secure Deployments](https://cloud.google.com/artifact-registry/docs/secure-deployments)
- [Google Cloud Upload SBOMs](https://cloud.google.com/artifact-analysis/docs/upload-sboms)
- [Anchore SBOM](https://anchore.com/platform/sbom/)

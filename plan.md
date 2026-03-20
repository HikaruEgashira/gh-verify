# gh-verify All-in-One SLSA Verification — 実装計画

## 設計方針

**Sigstore暗号検証は再実装しない。`gh attestation verify`をサブプロセスとして呼び出し、結果をControlとして統合する（転嫁戦略）。**

理由:
- Sigstore検証はセキュリティクリティカル。再実装はバグリスクが高い
- GitHub公式CLIの保守に追従するコストが不要
- `gh` CLIは`gh-verify`の前提依存（gh extension として配布）

## Phase 1: Build Track — アーティファクト来歴検証

### 1-1. Evidence拡張 (`crates/core/src/evidence.rs`)

`EvidenceBundle`にBuild Track用フィールドを追加:

```rust
pub struct ArtifactAttestation {
    pub subject: String,           // artifact path or OCI URI
    pub predicate_type: String,    // e.g. "https://slsa.dev/provenance/v1"
    pub signer_workflow: Option<String>,
    pub source_repo: Option<String>,
    pub verified: bool,
    pub verification_detail: Option<String>,
}

pub struct EvidenceBundle {
    pub change_requests: Vec<GovernedChange>,
    pub promotion_batches: Vec<PromotionBatch>,
    pub artifact_attestations: Vec<ArtifactAttestation>,  // NEW
}
```

### 1-2. ControlId拡張 (`crates/core/src/control.rs`)

```rust
pub enum ControlId {
    ReviewIndependence,
    SourceAuthenticity,
    BuildProvenance,       // NEW: アーティファクトにSLSA来歴attestationがあるか
}
```

### 1-3. BuildProvenanceControl (`crates/core/src/controls/build_provenance.rs`)

- `ArtifactAttestation`のverifiedフラグを検査
- 全アーティファクトがverified → Satisfied
- 一部未検証 → Violated
- attestationなし → NotApplicable (PR/Release検証時)

### 1-4. gh attestation verify 連携 (`crates/cli/src/attestation/`)

```rust
// crates/cli/src/attestation/gh_cli.rs
pub struct AttestationResult {
    pub verified: bool,
    pub predicate_type: String,
    pub signer_workflow: Option<String>,
    // ... parsed from `gh attestation verify --format json`
}

pub fn verify_artifact(artifact: &str, owner: &str) -> Result<Vec<AttestationResult>>
```

- `std::process::Command` で `gh attestation verify <artifact> --owner <owner> --format json` を実行
- JSON出力をパースして`AttestationResult`に変換
- `AttestationResult` → `ArtifactAttestation` へマッピング（adapter層）

### 1-5. CLI `artifact` サブコマンド (`crates/cli/src/main.rs`)

```rust
/// Verify artifact provenance
Artifact {
    /// Path to artifact or oci:// URI
    artifact: String,
    /// Owner or repo for attestation lookup
    #[arg(long)]
    owner: Option<String>,
    #[arg(long)]
    repo: Option<String>,
    #[arg(long, default_value = "human")]
    format: String,
}
```

### 1-6. Release統合

既存の`release`サブコマンドに`--artifact`オプション追加:
- 指定時: Source Track + Build Track の両方を検証
- 未指定: 従来通りSource Trackのみ

## Phase 2: Repository Security Controls

### 2-1. Evidence拡張

```rust
pub struct RepositoryPolicy {
    pub branch_protection: EvidenceState<BranchProtectionConfig>,
    pub codeowners: EvidenceState<bool>,  // CODEOWNERS file exists
    pub required_status_checks: EvidenceState<Vec<String>>,
}

pub struct BranchProtectionConfig {
    pub required_reviews: u32,
    pub dismiss_stale_reviews: bool,
    pub require_code_owner_reviews: bool,
    pub enforce_admins: bool,
    pub required_signatures: bool,
}

pub struct EvidenceBundle {
    pub change_requests: Vec<GovernedChange>,
    pub promotion_batches: Vec<PromotionBatch>,
    pub artifact_attestations: Vec<ArtifactAttestation>,
    pub repository_policy: EvidenceState<RepositoryPolicy>,  // NEW
}
```

### 2-2. 新Controls

| ControlId | 検証内容 |
|-----------|----------|
| `BranchProtection` | default branchのprotection rules |
| `RequiredReviewers` | 最低レビュー人数の設定 |

### 2-3. GitHub API

- `GET /repos/{owner}/{repo}/branches/{branch}/protection`
- レスポンスを`RepositoryPolicy`にマッピング

## Phase 3: SLSA Comprehensive Profile

### 3-1. 新Profile (`crates/core/src/profile.rs`)

```rust
pub struct SlsaComprehensiveProfile;
```

Source Track + Build Track の全Controlをカバーする上位プロファイル:

| Control | Satisfied | Violated | Indeterminate |
|---------|-----------|----------|---------------|
| ReviewIndependence | Pass | Fail | Fail |
| SourceAuthenticity | Pass | Fail | Fail |
| BuildProvenance | Pass | Fail | Review |
| BranchProtection | Pass | Fail | Review |
| RequiredReviewers | Pass | Fail | Review |

### 3-2. `--profile` フラグ

```
gh verify pr 42 --profile slsa-comprehensive
gh verify artifact ./binary --profile slsa-comprehensive
```

デフォルトは引き続き`slsa-foundation`。

## Phase 4: Creusot形式検証の拡張

### 4-1. 新述語 (`crates/verif/src/lib.rs`)

```rust
#[ensures(result == (attestation_count > 0 && all_verified))]
pub fn build_provenance_severity(attestation_count: u32, all_verified: bool) -> Severity

#[ensures(result == (required_reviews >= 1 && dismiss_stale && enforce_admins))]
pub fn branch_protection_severity(required_reviews: u32, dismiss_stale: bool, enforce_admins: bool) -> Severity
```

## 実装順序

```
Phase 1-1 → 1-2 → 1-3 → 1-4 → 1-5 → 1-6  (Build Track基盤)
     ↓
Phase 2-1 → 2-2 → 2-3                       (Repository Controls)
     ↓
Phase 3-1 → 3-2                              (統合Profile)
     ↓
Phase 4-1                                    (形式検証)
```

各Phaseは独立してリリース可能。Phase 1が最もインパクトが大きい。

## テスト戦略

- Core: `gh attestation verify` の出力JSONモック → `ArtifactAttestation`パース → Control評価
- CLI: `gh` コマンドのモック (テスト時は環境変数でスキップ可能)
- 形式検証: 新述語の exhaustive truth table テスト

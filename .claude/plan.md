# ADR-0002完遂: Branch Protection APIから事実ベース検証への移行

## 背景
ADR-0002で「Branch Protection APIを呼び出さない」と決定済みだが、実装が追いついていない。
2つのコントロールが`BranchProtectionEvidence`（admin権限必須API）に依存しており、
ほぼ全環境でindeterminateになっている。

## 方針
既にPR verifyで収集済みの`check_runs`, `approval_decisions`, `source_revisions`を使い、
事実ベースの検証に書き換える。

## 変更内容

### 1. `BranchHistoryIntegrity` (Source L2) — commit履歴の連続性チェックに変更
**現状**: `branch_protection.force_push_blocked / deletion_blocked`を見ている
**変更後**: `source_revisions`からcommit履歴の線形性を検証
- merge commitが含まれない = linear history (force-pushの形跡なし)
- source_revisionsが存在しない場合はindeterminate

integrityの`branch_history_severity`は引数の意味が変わる:
`unprotected_count` → `non_linear_count`（非線形コミットを持つchange requestの数）
関数のシグネチャ・ロジック自体は同一（0→Pass, ≥1→Error）なのでCreusotの#[ensures]は変更不要。

### 2. `BranchProtectionEnforcement` (Source L3) — 事実ベースの複合検証に変更
**現状**: `branch_protection`のreview/status check/admin設定を見ている
**変更後**: `check_runs` + `approval_decisions`の事実を複合検証
- check_runsが全てpassしている AND 独立したapprovalが存在する → Satisfied
- いずれかが欠けている → Violated
- evidenceが収集できていない → Indeterminate

integrityの`branch_protection_enforcement_severity`は引数の意味が変わる:
`non_enforced_count` → `violation_count`（技術的制御が事実として効いていないCRの数）
同様にシグネチャ・ロジック同一、Creusot変更不要。

### 3. `EvidenceBundle`から`branch_protection`フィールドを削除
- `BranchProtectionEvidence`型を削除
- `EvidenceBundle::branch_protection`フィールドを削除

### 4. CLIからBranch Protection API呼び出しを削除
- `main.rs`: `fetch_branch_protection_evidence()`呼び出しと関数定義を削除
- `repo_api.rs`: ファイルごと削除（ADR-0002の決定通り）
- `adapters/github.rs`: `map_branch_protection_evidence()`を削除
- `github/mod.rs`から`repo_api`のpub modを削除（あれば）

### 5. Creusot verif crateの更新
- `branch_history_severity`と`branch_protection_enforcement_severity`のコメントを更新
- ロジック・#[ensures]は変わらないため最小限の変更

### 6. ADR-0002のステータス更新は不要（既に「採用」）

## 実行順序
1. core: コントロール2つを書き換え + evidence型からBranchProtectionEvidence削除
2. cli: API呼び出し・アダプター・型を削除
3. verif: コメント更新
4. `devenv tasks run ghverify:test`で確認

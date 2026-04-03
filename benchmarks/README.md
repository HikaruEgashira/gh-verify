# ghverify Benchmarks

Benchmark suite for evaluating ghverify controls against real-world GitHub PRs.

## Running

```bash
devenv tasks run ghverify:bench
```

Results are saved to `results/run_<timestamp>.json`.

## Architecture

The benchmark runner executes **all controls** against each case via `assessment::assess`, then extracts the result for the specific control under test.

Each case specifies a `target_rule` field (e.g. `detect-unscoped-change`). The runner filters the assessment outcomes to that control's `control_id` and compares its decision against the case's `expected` severity. Cases without `target_rule` fall back to the max severity across all controls.

Multiple policy presets can be compared in a single run via `--algorithm`:

```bash
cargo run --bin gh-verify-bench -- --algorithm default,slsa-l1,oss
```

This produces a per-algorithm comparison table with accuracy and macro F1.

## Expanding With OSS Insight

Use OSS Insight to discover active repositories, then fetch recent merged PRs from GitHub:

```bash
cargo run --bin gh-verify-bench -- collect-real-world \
  --collection-id 10005 \
  --repo-limit 3 \
  --prs-per-repo 2 \
  --output benchmarks/discovery/ossinsight-real-world.json
```

The manifest records:

- OSS Insight ranking metadata for the selected collection
- top PR creators per repository
- recent merged PRs with observed ghverify severity and `merged_at`
- changed file paths and code file paths for each discovered PR, so curation can happen from one manifest

The collector uses OSS Insight for repository ranking and the authenticated `gh` CLI session for recent merged PR enumeration, so `gh auth status` must succeed before running it.

Curated benchmark cases should copy the relevant PRs into `cases/` with a `source` block so the provenance remains explicit.
New case `id` values should be stable descriptive slugs. Legacy severity-prefixed ids remain for historical continuity, but new ids should not encode the expected verdict.

## Detection Method: Call-Graph Connectivity

The `detect-unscoped-change` control (currently the only benchmarked control) extracts function definitions, function calls, and imports from each changed file's patch using tree-sitter AST analysis (with lexical fallback for unsupported languages). It builds a graph and checks connectivity using Union-Find:

- **Nodes**: changed files + extracted function definitions
- **Edges**: function call matches between files, import path resolution
- **Result**: number of connected components

| Components | Severity |
|------------|----------|
| 1 | `pass` |
| 2 | `warning` |
| 3+ | `error` |

Non-code files (`.md`, `.json`, `.yaml`, `.lock`, `.svg`, `.png`, etc.) are excluded from the graph.

## Metrics

The benchmark reports:

| Metric | Description |
|--------|-------------|
| **Accuracy** | Percentage of cases where actual == expected |
| **Precision** | Per-severity: correct predictions / total predictions for that severity |
| **Recall** | Per-severity: correct predictions / total expected for that severity |
| **F1** | Harmonic mean of precision and recall |

When cases target multiple rules, per-rule breakdowns are shown automatically.

## Case Structure

All benchmark cases are stored as flat JSON files in `cases/`.

| Field | Required | Description |
|-------|----------|-------------|
| `id` | yes | Stable case ID |
| `target_rule` | no | Control ID to evaluate (e.g. `detect-unscoped-change`). Omit for whole-assessment comparison. |
| `expected` | yes | Expected severity: `pass`, `warning`, or `error` |
| `category` | yes | Case category (see below) |
| `repo` | yes | GitHub `owner/repo` |
| `pr_number` | yes | PR number |

### Categories

| Category | Description |
|----------|-------------|
| connected-calls | Files connected via function call relationships |
| connected-imports | Files connected via import/require statements |
| disconnected-test | Test files disconnected from production code |
| disconnected-components | Multiple unrelated change clusters |
| multi-component | 3+ disconnected components |
| semantic-bridge | Implementation + tests/fixtures/snapshots that represent one semantic unit |
| fork-sync | Parallel build/runtime fork files that must move together |
| single-file | Single code file change |
| non-code-only | Only non-code files changed |

## Case Selection Criteria

1. **Executable**: Only merged PRs from public repositories
2. **Verified**: Each case tested with `gh verify pr <num> --repo <owner>/<repo> --format json`
3. **Diverse**: Multiple ecosystems (TypeScript, Python, Go, Rust)
4. **Educational**: Includes cases showing call-graph vs domain-based differences
5. **Traceable**: Real-world cases should retain discovery provenance (`source.provider`, collection, selection rule)

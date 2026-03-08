# ghverify Benchmarks

Benchmark suite for validating ghverify's `detect-unscoped-change` rule against real-world GitHub PRs.

## Running

```bash
devenv tasks run ghverify:bench
```

Results are saved to `results/run_<timestamp>.json`.

## Detection Method: Call-Graph Connectivity

The rule extracts function definitions, function calls, and imports from each changed file's patch using tree-sitter AST analysis (with lexical fallback for unsupported languages). It builds a graph and checks connectivity using Union-Find:

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

## Case Structure

All benchmark cases are stored as flat JSON files in `cases/`.

| Category | Description |
|----------|-------------|
| connected-calls | Files connected via function call relationships |
| connected-imports | Files connected via import/require statements |
| disconnected-test | Test files disconnected from production code |
| disconnected-components | Multiple unrelated change clusters |
| multi-component | 3+ disconnected components |
| single-file | Single code file change |
| non-code-only | Only non-code files changed |

## Case Selection Criteria

1. **Executable**: Only merged PRs from public repositories
2. **Verified**: Each case tested with `gh verify pr <num> --repo <owner>/<repo> --format json`
3. **Diverse**: Multiple ecosystems (TypeScript, Python, Go, Rust)
4. **Educational**: Includes cases showing call-graph vs domain-based differences

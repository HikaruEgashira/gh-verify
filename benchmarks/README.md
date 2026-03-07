# ghlint Benchmarks

Benchmark suite for validating ghlint's `detect-unscoped-change` rule against real-world GitHub PRs.

## Running

```bash
# Build ghlint
cd ..
zig build

# Run benchmarks
cd benchmarks
GHLINT_BIN=../zig-out/bin/gh-lint bash run.sh
```

Results are saved to `results/run_<timestamp>.json`.

## Case Structure

All benchmark cases are stored as flat JSON files in `cases/`.

| Prefix | Description |
|--------|-------------|
| pass-* | Single-domain or legitimately scoped PRs |
| warn-* | PRs spanning 2 unrelated domains |
| error-* | PRs spanning 3+ domains |

## Domain Classification Rules (`src/util/diff_parser.zig`)

| Domain | Detection Patterns |
|--------|-------------------|
| `test` | Path contains "test"/"spec", or `_test.*`/`.spec.*`/`.test.*` suffix |
| `ci` | Starts with `.github/`, or contains "ci"/"workflow" |
| `docs` | Starts with `docs/`, or `.md`/`.rst`/`.txt` suffix |
| `auth` | Contains "auth"/"login"/"token"/"session"/"oauth" |
| `database` | Contains "db"/"database"/"migration"/"schema", or `.sql` suffix |
| `ui` | Contains "ui"/"component"/"view"/"page", or `.css`/`.scss`/`.tsx`/`.jsx` suffix |
| `api` | Contains "api"/"handler"/"route"/"controller"/"endpoint" |
| `config` | Contains "config", or `.toml`/`.yaml`/`.yml`/`.env` suffix |
| `unknown` | No pattern matched (excluded from counting) |

**Priority**: First match wins (evaluated top to bottom).

**Special rules**:
- `test` domain is always ignored (not counted)
- `unknown` domain is also ignored
- Noise threshold: domains with ≤5 changed lines are ignored
- PASS: domain count ≤1, or exactly 2 domains with one being `docs`
- WARN: 2 unrelated domains
- ERROR: 3+ unrelated domains

## Known Issues (False Positives / False Negatives)

### False Positives

| Issue | Description | Example |
|-------|-------------|---------|
| Substring match | `token_parser` contains "token" → classified as `auth` | `stripe-sessions-contest.tsx` → `auth` |
| `schema` over-match | Zod/JSON schemas classified as `database` | `schemas.ts` → `database` |
| `session` over-match | Conference sessions classified as `auth` | `stripe-sessions-contest.tsx` |

### False Negatives

| Issue | Description | Example |
|-------|-------------|---------|
| `package.json` | `.json` matches no domain pattern → `unknown` | Dependency bump PRs always PASS |
| `Dockerfile` | Classified as `unknown` and ignored | Large infra changes still PASS |
| Case-sensitive dirs | `Auth/` doesn't match `auth` | Java/Kotlin repositories |
| Generic source files | `.go`/`.rs`/`.py` without domain path segments → `unknown` | Generic Go repositories |

## Case Selection Criteria

1. **Executable**: Only merged PRs from public repositories
2. **Verified**: Each case tested with `ghlint pr <num> --repo <owner>/<repo> --format json`
3. **Diverse**: Multiple ecosystems (TypeScript, Python, JavaScript)
4. **Educational**: Includes interesting false positive/negative cases

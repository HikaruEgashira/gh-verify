<h1 align="center">ghlint</h1>

<p align="center">
  A SLSA-based SDLC linter for GitHub pull requests.
</p>

<p align="center">
  <a href="HACKING.md">Hacking</a> · <a href="action/check-pr/README.md">GitHub Action</a> · <a href="benchmarks/README.md">Benchmarks</a>
</p>

---

**ghlint** checks whether a pull request follows healthy software development
lifecycle practices. It runs as a `gh` CLI extension and ships as a single
static binary built with Zig.

The tool analyzes PR diffs and metadata to detect anti-patterns — such as
changes that span too many unrelated domains — and reports them as
pass / warning / error with actionable suggestions.

> [!NOTE]
>
> This project is under active development. Rules and output format may change.

## Why?

Large, unfocused pull requests are hard to review, easy to mis-merge, and
a leading cause of subtle regressions. Automated scope checks catch these
problems before a reviewer has to.

ghlint enforces this at the PR level so teams get fast, consistent feedback
without relying solely on human judgement.

## Rules

| Rule | Severity | Description |
|---|---|---|
| `detect-unscoped-change` | warning / error | Flags PRs that touch multiple unrelated domains (auth, database, UI, etc.) |

Run `gh lint pr list-rules` to see all registered rules.

## Usage

### CLI

Requires: [GitHub CLI](https://cli.github.com/) (`gh`), Zig 0.15+.

```bash
# Build
zig build

# Lint a PR
gh lint pr 123 --repo owner/repo

# JSON output
gh lint pr 123 --repo owner/repo --format json

# List available rules
gh lint pr list-rules
```

### GitHub Action

Add to `.github/workflows/lint.yml`:

```yaml
on:
  pull_request:
    types: [opened, synchronize]

jobs:
  lint:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      pull-requests: read
    steps:
      - uses: HikaruEgashira/ghlint/action/check-pr@main
        with:
          pr-number: ${{ github.event.pull_request.number }}
```

See [action/check-pr](action/check-pr/README.md) for full input/output details.

## Exit Codes

- `0` — all rules pass (warnings are non-fatal)
- `1` — one or more rules returned an error

## Architecture

ghlint follows the Open/Closed Principle. Extending the tool requires
adding a new file and one line of registration — no changes to existing logic.

| Extension | Create | Register |
|---|---|---|
| New rule | `src/rules/<name>.zig` | 1 line in `engine.zig` `rules` array |
| New subcommand | `src/cli/<name>.zig` | 1 line in `main.zig` `dispatch_table` |
| New output format | `src/output/<name>.zig` | 1 case in `formatter.zig` switch |
| New API endpoint | `src/github/<name>.zig` | None |

## License

See [LICENSE](LICENSE).

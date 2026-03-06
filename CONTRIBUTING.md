# Contributing to ghlint

## Getting Started

See [HACKING.md](HACKING.md) for build and test instructions.

## Submitting Changes

- Open a pull request against `main`
- Keep each PR focused on a single concern — ghlint itself enforces this via the `detect-unscoped-change` rule
- All tests must pass: `bash tests/run.sh`

## Adding a Rule

Rules are self-contained. The steps are:

1. Create `src/rules/<name>.zig` — export `pub fn run(alloc, ctx) ![]RuleResult`
2. Add one line to `src/rules/engine.zig` — append to the `rules` array
3. Add a test case or document expected behavior in `tests/`
4. Document the rule in `README.md`

No other files need to change.

## Code Style

- Zig standard formatting: run `zig fmt src/` before committing
- No external dependencies — zero is the target
- Each file has a single, stated responsibility (see architecture in `CLAUDE.md`)

## Reporting Issues

Open a GitHub issue with:
- Zig version (`zig version`)
- Command run and full output
- Expected vs actual behavior

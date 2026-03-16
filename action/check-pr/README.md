# action/check-pr

GitHub Action that runs `gh-verify` SDLC checks on a pull request.

## Usage

```yaml
on:
  pull_request:
    types: [opened, synchronize]

jobs:
  verify:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      pull-requests: read
    steps:
      - uses: HikaruEgashira/gh-verify/action/check-pr@main
        with:
          pr-number: ${{ github.event.pull_request.number }}
```

### Pin to a specific version

```yaml
      - uses: HikaruEgashira/gh-verify/action/check-pr@main
        with:
          pr-number: ${{ github.event.pull_request.number }}
          version: "v0.3.0"
```

## Inputs

| Input | Required | Default | Description |
|---|---|---|---|
| `pr-number` | yes | — | PR number to verify |
| `repo` | no | current repo | `OWNER/REPO` format |
| `format` | no | `human` | `human` or `json` |
| `version` | no | latest | gh-verify version to install |

## Outputs

| Output | Description |
|---|---|
| `result` | `pass`, `warning`, or `error` |

## Exit Codes

- `0`: all rules pass (warnings are non-fatal)
- `1`: one or more rules returned an error

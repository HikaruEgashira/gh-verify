# action/check-pr

GitHub Action that runs `gh-lint` SDLC checks on a pull request.

## Usage

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

## Inputs

| Input | Required | Default | Description |
|---|---|---|---|
| `pr-number` | yes | — | PR number to lint |
| `repo` | no | current repo | `OWNER/REPO` format |
| `format` | no | `human` | `human` or `json` |
| `rule` | no | (all) | Run only a specific rule ID |

## Outputs

| Output | Description |
|---|---|
| `result` | `pass`, `warning`, or `error` |

## Exit Codes

- `0`: all rules pass (warnings are non-fatal)
- `1`: one or more rules returned an error

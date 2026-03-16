# action/check-pr

GitHub Action that runs `gh-verify` SDLC checks on a pull request.

## Usage

```yaml
on:
  pull_request:
    types: [opened, synchronize, reopened]
  pull_request_review:
    types: [submitted]

concurrency:
  group: check-pr-${{ github.event.pull_request.number }}
  cancel-in-progress: true

jobs:
  verify:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      pull-requests: read
    steps:
      - uses: actions/checkout@v4
        with:
          ref: ${{ github.event.pull_request.head.sha }}
      - uses: HikaruEgashira/gh-verify/action/check-pr@main
        with:
          pr-number: ${{ github.event.pull_request.number }}
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

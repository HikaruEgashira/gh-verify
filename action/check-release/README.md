# action/check-release

GitHub Action that runs `gh-verify` SDLC checks on a release.
Designed as a pre-build gate in release workflows.

## Usage

```yaml
on:
  push:
    tags: ["v*"]

jobs:
  verify:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: HikaruEgashira/gh-verify/action/check-release@main
        with:
          tag: ${{ github.ref_name }}

  build:
    needs: verify
    # ...
```

## Inputs

| Input | Required | Default | Description |
|---|---|---|---|
| `tag` | yes | — | Release tag or range (`v1.0.0` or `v0.9.0..v1.0.0`) |
| `repo` | no | current repo | `OWNER/REPO` format |
| `format` | no | `human` | `human` or `json` |
| `version` | no | latest | gh-verify version to install |

## Outputs

| Output | Description |
|---|---|
| `result` | `pass`, `warning`, or `error` |

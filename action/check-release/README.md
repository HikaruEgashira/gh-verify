# action/check-release

GitHub Action that runs `gh-verify` SDLC checks on a release.

## Usage

```yaml
on:
  release:
    types: [published]

jobs:
  verify:
    runs-on: ubuntu-latest
    permissions:
      contents: read
    steps:
      - uses: HikaruEgashira/gh-verify/action/check-release@main
        with:
          tag: ${{ github.event.release.tag_name }}
```

### Tag range verification

```yaml
      - uses: HikaruEgashira/gh-verify/action/check-release@main
        with:
          tag: "v0.9.0..v1.0.0"
```

## Inputs

| Input | Required | Default | Description |
|---|---|---|---|
| `tag` | yes | — | Release tag or range (`v1.0.0` or `v0.9.0..v1.0.0`) |
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

# Post-merge self-healing

Install [hikae-selfheal](https://github.com/apps/hikae-selfheal) to handle default-branch CI failures. Repository workflows and secrets are not required. The App creates a repair PR, validates it with native CI, merges it after successful validation, and closes the failure issue after successful main-branch CI.

Disable and remove any legacy `.github/workflows/self-heal.yml` before App cutover to avoid duplicate repairs. See the [App operations guide](https://github.com/HikaruEgashira/hikae-selfheal#operations) for permissions, supported owners and recovery behavior.

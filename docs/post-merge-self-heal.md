# Post-merge self-healing

The reusable `post-merge-heal.yml` workflow observes failed pushes and manual deployments on the default branch of `HikaruEgashira` and `plenoai` repositories. It creates an issue, then an ephemeral `hikae-economy` runner uses Pi to prepare one production-source repair PR from the latest default branch.

Pin both the reusable workflow and `automation_ref` to the same immutable commit. Supply `check_workflows` as a JSON list of existing workflows that accept `workflow_dispatch` and safely test a repair branch. Keep deployment jobs restricted to the default branch. Supply `post_merge_workflows` as the JSON list of workflows to dispatch on the default branch after a repair merge.

The agent receives the model key, without the repository write token or deployment credentials. It cannot submit changes to automation, dependency manifests, standalone tests, or Terraform state. Existing CI must pass on the exact repair SHA, with at least one completed job. A missing validation workflow leaves the PR for review. A newer default-branch revision prevents stale patches from merging. Branch protection remains in force.

A successful validation is followed by squash merge and explicit dispatch of the default-branch workflows: a `GITHUB_TOKEN` merge does not trigger push workflows. Issues close only when the same workflow succeeds on the failed or a newer revision. Failed repairs retain the issue and PR; repair revisions never start a recursive repair.

Prerequisites: the `hikae-runners` GitHub App must cover the repository, and Actions must be allowed to create pull requests. No cloud credentials or model secrets need to be copied into each caller.

Run the offline regression checks with `python3 .github/self-heal/test_heal.py`.

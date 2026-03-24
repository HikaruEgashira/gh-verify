# Package Registry Provenance: Who Signs What, and Why It Matters

Your lock file pins versions. But who published them? From which source? Can you verify that the artifact you downloaded was actually built from the commit it claims?

These questions sit at the heart of **software supply chain security**. The answers vary wildly depending on your package registry.

## The SLSA Dependencies Track

[SLSA v1.2](https://slsa.dev/spec/v1.0/levels) defines four levels for dependency verification:

| Level | What It Proves | Mechanism |
|-------|----------------|-----------|
| **L1** | Integrity — artifact hasn't been tampered with | Checksum in lock file |
| **L2** | Provenance — artifact was built from a known source | Cryptographic signature + `source_repo` |
| **L3** | Auditability — signer identity is verifiable and publicly logged | Signer identity + transparency log (Rekor) |
| **L4** | Completeness — the entire dependency tree (direct + transitive) meets L3 | Full tree verification |

Every level builds on the previous. L1 is table stakes; L4 is aspirational for most ecosystems today.

## The Landscape (Early 2026)

### npm — L3: Full Trust Chain

npm is the clear leader in dependency provenance. Since GA in October 2023, npm leverages **Sigstore keyless signing** and the **Rekor transparency log** to provide verifiable build provenance.

**What works today:**
- `npm audit signatures` verifies Sigstore attestations
- Provenance links packages to their GitHub Actions / GitLab CI build
- Signer identity (OIDC subject) and transparency log entry are both available
- Over 500 million downloads of provenance-enabled package versions during beta alone

**What this means for `gh-verify`:**
- `dependency-provenance` (L2): **enforced** — npm packages should have cryptographic provenance
- `dependency-signer-verified` (L3): **enforced** — signer identity and Rekor log are available
- `dependency-completeness` (L4): **enforced** — full tree verification is feasible

If your npm dependency lacks provenance, that's a real signal worth investigating.

### PyPI — L2: Cryptographic Provenance (L3 Growing)

PyPI has made remarkable progress since launching **Trusted Publishers** and **Sigstore-based attestations**.

**What works today:**
- [Trusted Publishers](https://docs.pypi.org/trusted-publishers/) — OIDC-based authentication for CI/CD publishing
- Automatic attestation generation via `pypa/gh-action-pypi-publish`
- **17% of all uploads** now include attestations
- **132,360+ packages** have attestations (as of March 2026)
- The [Ultralytics supply-chain attack](https://blog.pypi.org/posts/2025-12-31-pypi-2025-in-review/) was detectable precisely because the project used digital attestations

**What's still maturing:**
- Signer identity coverage is partial — not all attestation-enabled packages expose full identity
- Transparency log integration is less standardized than npm's Rekor usage
- Adoption, while growing fast (17%), means 83% of uploads still lack attestations

**What this means for `gh-verify`:**
- `dependency-provenance` (L2): **enforced** — PyPI packages with Trusted Publishers should have attestations
- `dependency-signer-verified` (L3): **not yet enforced** — signer identity coverage is inconsistent
- `dependency-completeness` (L4): **not yet enforced** — depends on L3

### crates.io — L1: Integrity Only

The Rust ecosystem provides strong integrity guarantees but lacks cryptographic provenance infrastructure.

**What works today:**
- SHA-256 checksums in `Cargo.lock` — every dependency is checksum-pinned
- [Trusted Publishing (RFC #3691)](https://rust-lang.github.io/rfcs/3691-trusted-publishing-cratesio.html) — authentication for CI-based publishing (auth only, not signing)
- Git dependency commit pinning (`source = "git+...#<sha>"`) provides integrity for non-registry dependencies

**What's missing:**
- No cryptographic signatures on published crates
- No provenance attestations linking crates to their source repository
- No transparency log integration
- If crates.io or the index were compromised, Cargo would not detect it as long as checksums match

**What this means for `gh-verify`:**
- `dependency-signature` (L1): **enforced** — checksum verification works
- `dependency-provenance` (L2): **not applicable** — infrastructure doesn't exist yet
- `dependency-signer-verified` (L3): **not applicable**
- `dependency-completeness` (L4): **not applicable**

## How gh-verify Handles This

Rather than producing hundreds of false positives for ecosystems that lack provenance infrastructure, `gh-verify` scopes each control to registries that can actually satisfy it:

```
$ gh verify repo
[dependency-signature] pass: All 267 deps verified (263 checksum, 4 git-commit-pin)
```

For a pure Rust project, L2/L3/L4 controls are `NotApplicable` — not failed. No noise, no wasted triage time.

For a Node.js project, those same controls are enforced — and violations are real signals:

```
$ gh verify repo --repo expressjs/express
[dependency-signature]      pass: All 48 deps verified (48 checksum)
[dependency-provenance]     fail: lodash@4.17.21 (no cryptographic signature, no source_repo)
[dependency-signer-verified] fail: lodash@4.17.21 (no signer_identity, no transparency_log)
```

This is the difference between a tool that produces actionable findings and one that produces a wall of noise.

## The Trajectory

Supply chain security is moving fast:

- **npm** led the way and is approaching ecosystem-wide coverage
- **PyPI** crossed the tipping point in 2025 with 17% attestation adoption and growing
- **crates.io** has the auth foundation (Trusted Publishing) but signing remains on the roadmap
- **Maven Central** mandated PGP signatures in 2024
- **Go** has `sumdb` for module transparency but not Sigstore-style provenance

Within 2-3 years, we expect L2 provenance to be table stakes across all major registries. `gh-verify`'s registry capability model will promote registries from `ChecksumOnly` → `CryptographicProvenance` → `FullTrustChain` as infrastructure matures, automatically enforcing stricter controls without changing policy.

## Try It

```bash
gh extension install HikaruEgashira/gh-verify
gh verify repo                           # Check your dependencies
gh verify repo --policy slsa-l2          # Enforce SLSA L2 for provenance-capable deps
gh verify pr 123 --repo org/repo         # Verify a pull request
```

Sources:
- [npm provenance GA announcement](https://blog.sigstore.dev/npm-provenance-ga/)
- [PyPI attestations GA](https://blog.sigstore.dev/pypi-attestations-ga/)
- [PyPI 2025 Year in Review](https://blog.pypi.org/posts/2025-12-31-pypi-2025-in-review/)
- [crates.io Trusted Publishing RFC #3691](https://rust-lang.github.io/rfcs/3691-trusted-publishing-cratesio.html)
- [SLSA Specification](https://slsa.dev/spec/v1.0/levels)

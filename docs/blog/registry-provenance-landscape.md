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

**Attestation API:**
```bash
curl -s 'https://registry.npmjs.org/-/npm/v1/attestations/semver@7.6.3' | jq '.attestations[].predicateType'
# "https://github.com/npm/attestation/tree/main/specs/publish/v0.1"
# "https://slsa.dev/provenance/v1"
```

The SLSA provenance payload contains:
- `buildDefinition.externalParameters.workflow.repository` → source repo
- `resolvedDependencies[0].digest.gitCommit` → source commit
- Sigstore certificate SAN → signer identity (workflow URI)
- `verificationMaterial.tlogEntries[0].logIndex` → Rekor transparency log

**Real-world result with `gh-verify`:**
```
$ gh verify repo --repo sigstore/sigstore-js
Fetching npm provenance for 1059 packages (16 concurrent)...
  76/1059 npm packages have provenance attestations

[dependency-signature]       fail: 547 unverified (attestation_absent)
[dependency-provenance]      fail: 983 lacking provenance
[dependency-signer-verified] fail: 983 lacking signer verification
```

Even in the Sigstore project itself, only 7.2% of npm dependencies have provenance — showing both how the controls produce real, actionable signals and how far ecosystem adoption still needs to go.

**What this means for `gh-verify`:**
- `dependency-provenance` (L2): **enforced** — npm packages should have cryptographic provenance
- `dependency-signer-verified` (L3): **enforced** — signer identity and Rekor log are available
- `dependency-completeness` (L4): **enforced** — full tree verification is feasible

If your npm dependency lacks provenance, that's a real signal worth investigating.

### PyPI — L3: Full Trust Chain (Same Stack as npm)

PyPI uses the **same Sigstore stack** (Fulcio + Rekor) as npm. Packages with attestations provide the full L3 trust chain.

**What works today:**
- [Trusted Publishers](https://docs.pypi.org/trusted-publishers/) — OIDC-based authentication for CI/CD publishing
- Automatic attestation generation via `pypa/gh-action-pypi-publish`
- **17% of all uploads** now include attestations
- **132,360+ packages** have attestations (as of March 2026)
- The [Ultralytics supply-chain attack](https://blog.pypi.org/posts/2025-12-31-pypi-2025-in-review/) was detectable precisely because the project used digital attestations

**Attestation API ([PEP 740](https://peps.python.org/pep-0740/)):**
```bash
curl -s -H "Accept: application/vnd.pypi.integrity.v1+json" \
  'https://pypi.org/integrity/cryptography/44.0.0/cryptography-44.0.0.tar.gz/provenance'
```

Returns:
- `publisher.repository` → source repo (e.g. `pyca/cryptography`)
- `publisher.workflow` → CI workflow file (signer identity)
- `verification_material.transparency_entries[].logIndex` → Rekor entry
- `verification_material.certificate` → Fulcio cert (signer identity in X.509 SAN)

**npm vs PyPI — the key difference is adoption model, not capability:**
- npm: registry-initiated signing (npm registry generates attestation at publish time)
- PyPI: publisher-initiated signing (CI/CD must use Trusted Publishing + `pypa/gh-action-pypi-publish`)
- Both produce identical L3 artifacts: Sigstore signature + signer identity + Rekor transparency log

**Real-world result with `gh-verify`:**
```
$ gh verify repo --repo python-poetry/poetry
Fetching PyPI provenance for 98 packages (16 concurrent)...
  32/98 PyPI packages have provenance attestations

[dependency-provenance]      fail: 66 lacking provenance
[dependency-signer-verified] fail: 66 lacking signer verification
```

32.7% of poetry's dependencies have provenance — significantly higher than npm's 7.2% in sigstore-js, reflecting PyPI's faster adoption trajectory.

**What this means for `gh-verify`:**
- `dependency-provenance` (L2): **enforced**
- `dependency-signer-verified` (L3): **enforced**
- `dependency-completeness` (L4): **enforced**

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

### Maven Central — L3 Capability (Sigstore Opt-in, PGP Mandatory)

Maven Central added [Sigstore signature validation](https://socket.dev/blog/maven-central-adds-sigstore-signature-validation) in January 2025. PGP signatures remain mandatory. Sigstore is opt-in and provides full L3 when present.

**What works today:**
- PGP signatures (`.asc`) are mandatory for all published artifacts
- Sigstore bundles (`.sigstore.json`) are validated by the Maven Central Publisher Portal (since Jan 2025)
- `sigstore-java` enables publishers to sign with Sigstore

**PGP verification (L1 — all artifacts):**
```bash
curl -O 'https://repo1.maven.org/maven2/com/google/guava/guava/33.0.0-jre/guava-33.0.0-jre.jar'
curl -O 'https://repo1.maven.org/maven2/com/google/guava/guava/33.0.0-jre/guava-33.0.0-jre.jar.asc'
gpg --verify guava-33.0.0-jre.jar.asc guava-33.0.0-jre.jar
```

**Sigstore verification (L3 — opt-in, rare):**
```bash
curl -s 'https://repo1.maven.org/maven2/dev/sigstore/sigstore-java/1.0.0/sigstore-java-1.0.0.pom.sigstore.json'
# Returns standard Sigstore bundle with Fulcio cert + Rekor transparency log
```

**Current limitations:**
- PGP gives a key ID but not a clean identity chain (no OIDC, no CI provenance)
- Sigstore bundles are available but extremely rare (only early adopters like sigstore-java itself)
- No dedicated query API — must try URL conventions (`{artifact}.sigstore.json`) and handle 404s
- Sonatype has ["no intention of replacing PGP"](https://central.sonatype.org/news/20220310_sigstore/) but may consider it if Sigstore adoption grows

### Go Modules — L1: Checksum Transparency (No Provenance)

Go has the **strongest integrity guarantee** through its checksum database, but **zero provenance information** at the registry level.

**What works today:**
- [sum.golang.org](https://sum.golang.org) — a tamper-evident log of module checksums
- Every module download is verified against this log
- Merkle tree inclusion proofs guarantee global consistency

**API:**
```bash
curl -s 'https://sum.golang.org/lookup/golang.org/x/text@v0.14.0'
# Returns checksum lines + signed tree head
```

**What's missing at the registry:**
- No signer identity — you cannot determine who published a module
- No source provenance — no link to the repository that built it
- No Sigstore integration at proxy.golang.org

**Workaround via GitHub Artifact Attestations:**
Go binaries built with GitHub Actions can achieve [SLSA L3 via `slsa-framework/slsa-github-generator`](https://github.blog/security/supply-chain-security/slsa-3-compliance-with-github-actions/). This is a build-level attestation for the compiled binary, not the Go module itself. Tools like mise verify these attestations when installing Go CLI tools.

### GitHub Releases — L3: Platform-Level Provenance

GitHub is not a package registry, but its **Artifact Attestations** provide L3 provenance for release binaries across any language ecosystem.

**What works today:**
- [`actions/attest-build-provenance`](https://github.com/actions/attest-build-provenance) — generates SLSA provenance in GitHub Actions
- Sigstore signing (public-good instance for public repos, GitHub private instance for private repos)
- `gh attestation verify <artifact>` — CLI verification
- SLSA v1.0 Build Level 2 out of the box; Level 3 with reusable workflows

**Why this matters:**
- Language-agnostic: works for Go binaries, Rust binaries, Java JARs, anything
- Fills the gap for ecosystems where the registry lacks provenance (crates.io, Go)
- Tools like [mise](https://mise.jdx.dev/) already consume GitHub Artifact Attestations to verify downloaded tool binaries

**gh-verify integration:**
The `gh verify release` command already verifies GitHub Artifact Attestations for release assets.

## Comparison Matrix

| Capability | npm | PyPI | GitHub Releases | Maven Central | crates.io | Go |
|---|---|---|---|---|---|---|
| **L1** Checksum integrity | SRI hash | SHA-256 | SHA-256 | Artifact hash | SHA-256 | go.sum |
| **L2** Source provenance | Sigstore | Sigstore | Sigstore | PGP key ID | None | None |
| **L3** Signer identity | OIDC + Rekor | Fulcio + Rekor | Fulcio + Rekor | Rekor (rare) | None | None |
| Transparency log | Rekor | Rekor | Rekor | Rekor (rare) | None | sum.golang.org |
| Dedicated API | Yes | Yes (PEP 740) | Yes (`gh attestation`) | No | No | Yes |
| L3 coverage | ~7% | ~17% | Opt-in (growing) | <1% | 0% | 0% |

## How gh-verify Handles This

Rather than producing hundreds of false positives for ecosystems that lack provenance infrastructure, `gh-verify` scopes each control to registries that can actually satisfy it:

```
$ gh verify repo                  # Pure Rust project
[dependency-signature] pass: All 267 deps verified (263 checksum, 4 git-commit-pin)

Summary: 1 pass, 0 review, 0 fail
```

For a pure Rust project, L2/L3/L4 controls are `NotApplicable` — not failed. No noise, no wasted triage time.

For a Node.js project, `gh-verify` fetches provenance from the npm attestation API in parallel (16 concurrent) and produces actionable findings:

```
$ gh verify repo --repo sigstore/sigstore-js
Fetching npm provenance for 1059 packages (16 concurrent)...
  76/1059 npm packages have provenance attestations

[dependency-signature]       fail: 547 unverified
[dependency-provenance]      fail: 983 lacking provenance
[dependency-signer-verified] fail: 983 lacking signer verification

Summary: 0 pass, 0 review, 3 fail
```

## The Trajectory

The supply chain security landscape is converging on **Sigstore as the common standard**:

- **npm** (L3) — led the way, approaching ecosystem-wide coverage
- **PyPI** (L3) — crossed the tipping point with 17% attestation adoption and accelerating
- **GitHub Releases** (L3) — language-agnostic provenance for any binary built on Actions
- **Maven Central** (L3 capable) — Sigstore validation added Jan 2025, adoption nascent
- **crates.io** (L1) — Trusted Publishing for auth, Sigstore RFC #3403 proposed
- **Go** (L1) — strongest integrity (sum.golang.org), but provenance relies on GitHub Actions
- **NuGet** (L1) — X.509 signing exists, provenance via GitHub Artifact Attestations

The pattern is clear: **Sigstore (Fulcio + Rekor)** is becoming the universal provenance layer. Ecosystems that adopt it get L3 immediately. The remaining gap is adoption rate, not infrastructure.

Tools like [mise](https://mise.jdx.dev/) already verify SLSA provenance and GitHub Artifact Attestations for downloaded tool binaries, making provenance verification transparent to developers.

`gh-verify`'s registry capability model promotes registries from `ChecksumOnly` → `FullTrustChain` as they adopt Sigstore, automatically enforcing stricter controls without changing policy.

## Try It

```bash
gh extension install HikaruEgashira/gh-verify
gh verify repo                           # Check your dependencies
gh verify repo --policy slsa-l2          # Enforce SLSA L2 for provenance-capable deps
gh verify pr 123 --repo org/repo         # Verify a pull request
```

Sources:
- [npm provenance GA announcement](https://blog.sigstore.dev/npm-provenance-ga/)
- [npm attestation API](https://docs.npmjs.com/generating-provenance-statements/)
- [PyPI attestations GA](https://blog.sigstore.dev/pypi-attestations-ga/)
- [PyPI 2025 Year in Review](https://blog.pypi.org/posts/2025-12-31-pypi-2025-in-review/)
- [PyPI Integrity API (PEP 740)](https://docs.pypi.org/attestations/)
- [GitHub Artifact Attestations](https://docs.github.com/en/actions/security-for-github-actions/using-artifact-attestations/using-artifact-attestations-to-establish-provenance-for-builds)
- [SLSA L3 with GitHub Actions](https://github.blog/security/supply-chain-security/slsa-3-compliance-with-github-actions/)
- [Maven Central Sigstore validation](https://socket.dev/blog/maven-central-adds-sigstore-signature-validation)
- [crates.io Trusted Publishing RFC #3691](https://rust-lang.github.io/rfcs/3691-trusted-publishing-cratesio.html)
- [crates.io Sigstore RFC #3403](https://github.com/rust-lang/rfcs/pull/3403)
- [Go Checksum Database](https://go.dev/ref/mod#checksum-database)
- [mise SLSA/attestation verification](https://mise.jdx.dev/configuration/settings.html)
- [SLSA Specification v1.2](https://slsa.dev/spec/v1.0/levels)

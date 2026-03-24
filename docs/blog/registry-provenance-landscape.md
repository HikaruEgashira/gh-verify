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

### PyPI — L2: Cryptographic Provenance (L3 Growing)

PyPI has made remarkable progress since launching **Trusted Publishers** and **Sigstore-based attestations**.

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
- `publisher.workflow` → CI workflow file
- `verification_material.transparency_entries[].logIndex` → Rekor entry
- `verification_material.certificate` → Fulcio cert (signer identity in X.509 SAN)

**What's still maturing:**
- Signer identity coverage is partial — not all attestation-enabled packages expose full identity
- Adoption, while growing fast (17%), means 83% of uploads still lack attestations
- Only packages published via Trusted Publishing have attestations (legacy uploads don't)

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

### Maven Central — L1 + PGP (Sigstore Optional)

Maven Central mandated PGP signatures in 2024. All artifacts must have a detached `.asc` signature file.

**What works today:**
- PGP signatures (`.asc`) are mandatory for all published artifacts
- Sigstore bundles (`.sigstore.json`) are supported but opt-in

**PGP verification:**
```bash
# Download artifact + signature
curl -O 'https://repo1.maven.org/maven2/com/google/guava/guava/33.0.0-jre/guava-33.0.0-jre.jar'
curl -O 'https://repo1.maven.org/maven2/com/google/guava/guava/33.0.0-jre/guava-33.0.0-jre.jar.asc'
gpg --verify guava-33.0.0-jre.jar.asc guava-33.0.0-jre.jar
```

**Sigstore bundles (rare):**
```bash
curl -s 'https://repo1.maven.org/maven2/dev/sigstore/sigstore-java/1.0.0/sigstore-java-1.0.0.pom.sigstore.json'
# Returns standard Sigstore bundle with Rekor transparency log entry
```

**Limitations:**
- PGP gives you a key ID but not a clean identity chain (no OIDC, no CI provenance)
- Sigstore bundles are available but extremely rare
- No dedicated query API — must try URL conventions and handle 404s
- No batch/bulk verification endpoint

### Go Modules — Checksum Transparency (No Provenance)

Go has the **strongest integrity guarantee** through its checksum database, but **zero provenance information**.

**What works today:**
- [sum.golang.org](https://sum.golang.org) — a tamper-evident log of module checksums
- Every module download is verified against this log
- Merkle tree inclusion proofs guarantee global consistency

**API:**
```bash
curl -s 'https://sum.golang.org/lookup/golang.org/x/text@v0.14.0'
# Returns checksum lines + signed tree head
```

**What's missing:**
- No signer identity — you cannot determine who published a module
- No source provenance — no link to the repository that built it
- No build attestation — no CI/CD workflow information
- No Sigstore integration

Go proves that everyone gets the same source code, but says nothing about who authored it or how it was built.

## Comparison Matrix

| Capability | npm | PyPI | crates.io | Maven Central | Go |
|---|---|---|---|---|---|
| **L1** Checksum integrity | SRI hash | SHA-256 | SHA-256 | Artifact hash | go.sum |
| **L2** Source provenance | Sigstore | Sigstore | None | PGP key ID | None |
| **L3** Signer identity | OIDC + Rekor | Fulcio cert | None | PGP (weak) | None |
| Transparency log | Rekor | Rekor | None | Rekor (rare) | sum.golang.org |
| Dedicated API | Yes | Yes (PEP 740) | No | No | Yes |
| Coverage | ~7% of deps | ~17% of uploads | 100% (L1) | 100% PGP | 100% (L1) |

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

Supply chain security is moving fast:

- **npm** led the way and is approaching ecosystem-wide coverage
- **PyPI** crossed the tipping point in 2025 with 17% attestation adoption and growing
- **crates.io** has the auth foundation (Trusted Publishing) but signing remains on the roadmap
- **Maven Central** has universal PGP signatures but weak identity chains; Sigstore adoption is nascent
- **Go** has the strongest integrity guarantees but no provenance at all

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
- [npm attestation API](https://docs.npmjs.com/generating-provenance-statements/)
- [PyPI attestations GA](https://blog.sigstore.dev/pypi-attestations-ga/)
- [PyPI 2025 Year in Review](https://blog.pypi.org/posts/2025-12-31-pypi-2025-in-review/)
- [PyPI Integrity API (PEP 740)](https://docs.pypi.org/attestations/)
- [crates.io Trusted Publishing RFC #3691](https://rust-lang.github.io/rfcs/3691-trusted-publishing-cratesio.html)
- [Maven Central Sigstore support](https://central.sonatype.org/news/20220310_sigstore/)
- [Go Checksum Database](https://go.dev/ref/mod#checksum-database)
- [SLSA Specification](https://slsa.dev/spec/v1.0/levels)

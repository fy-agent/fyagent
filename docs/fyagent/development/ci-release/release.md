# Release flow

The current `.github/workflows/release.yml`, release scripts, Cargo workspace
version, and executable contract tests define source eligibility, native build
topology, asset/evidence aggregation, attestation, publication, canonical
SemVer, and asset names. Retained release-workflow and application-version
notes under `.trellis/spec/` are optional AI-assistance review material.

## Exact-source progression

```text
merge into main
  -> current remote main HEAD
  -> successful full push CI for that exact SHA
  -> annotated stable tag targeting that exact SHA
  -> one formal native build and evidence workflow
  -> private draft upload and re-download verification
  -> one final publication transition
```

Formal mode requires the current remote `main` source and successful push-CI
evidence for that exact SHA. A dispatch preflight remains available for the
current remote `dev/laiyongjie` HEAD after its exact push CI succeeds, but it is
an artifact-producing diagnostic rather than a release-closure prerequisite,
and its publication condition is always false. The shortest authoritative path is
therefore `main` merge -> exact-SHA `CI / Required` success -> annotated tag ->
one formal build. A formal run refuses a lightweight tag, a moved `main`, stale
green CI, identity mismatch, partial signer configuration, incomplete native
evidence, or asset drift.

Platform acceptance is successful build and packaging on each matching native
runner. Windows additionally requires strict unsigned/signing proof and the
fresh formal sealing boundary before exact-asset verification. The Release
workflow does not launch the setup executables or run an install -> verify ->
uninstall lifecycle; the retained lifecycle harness is a manual diagnostic,
not a preflight or publication gate.

Release metadata retains real schema identities for download manifests,
platform builds, aggregate build metadata, and Windows signing status. The
Windows signing table in public notes is generated from verified metadata;
credentials and provider commands never become documentation or attachments.

Optional task archive or journal bookkeeping is outside the Release trust
chain. It does not create release eligibility, move a release tag, or replace
the required exact-source CI and workflow evidence.

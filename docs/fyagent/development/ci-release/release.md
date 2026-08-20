# Release flow

The current `.github/workflows/release.yml`, release scripts, Cargo workspace
version, and executable contract tests define source eligibility, native build
topology, asset/evidence aggregation, attestation, publication, canonical
SemVer, and asset names. Retained release-workflow and application-version
notes under `.trellis/spec/` are optional AI-assistance review material.

## Exact-source progression

```text
tag vX.Y.Z at the intended commit
  -> tag target SHA is the frozen source (annotated or lightweight)
  -> one formal native build and evidence workflow
  -> private draft upload and re-download verification
  -> one final publication transition
```

Formal mode binds the remote tag's target commit. Live `main` may move during
the run. Exact-source push CI is not required; the Release compile is the
proof. A dispatch preflight remains available for the current remote
`dev/laiyongjie` HEAD, but it is an artifact-producing diagnostic rather than a
release-closure prerequisite, and its publication condition is always false.
The shortest authoritative path is therefore a matching stable tag -> one
formal build. A formal run refuses a tag whose target is not the frozen SHA,
an existing draft or published GitHub Release, identity mismatch, partial
signer configuration, incomplete native evidence, or asset drift. Operators
may force-update `vX.Y.Z` when no GitHub Release exists for that tag.

Platform acceptance is successful build and packaging on each matching native
runner. Windows additionally requires strict unsigned/signing proof and the
fresh formal sealing boundary before exact-asset verification. macOS
additionally requires Developer ID signing, one Apple notarization of the
signed DMG, a stapled DMG ticket, and a stapled ticket on the original app.
The published macOS installer is that DMG, with `FyAgent.app` and an
`Applications` symlink so Finder can drag-install. The notarization helper
submits the DMG once, then polls `notarytool info`
until Apple returns a terminal status, instead of treating a
`notarytool wait` timeout as rejection. The
Release workflow does not launch the setup
executables or run an install -> verify -> uninstall lifecycle; the retained
lifecycle harness is a manual diagnostic, not a preflight or publication gate.

Release metadata retains real schema identities for download manifests,
platform builds, aggregate build metadata, and Windows signing status. The
Windows signing table in public notes is generated from verified metadata;
credentials and provider commands never become documentation or attachments.

Optional task archive or journal bookkeeping is outside the Release trust
chain. It does not create release eligibility, move a release tag, or replace
the tagged source SHA and workflow evidence.

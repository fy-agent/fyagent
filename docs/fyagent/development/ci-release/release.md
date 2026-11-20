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
candidate SHA: the workflow is dispatched from trusted `main`, while
`source_sha` selects the immutable commit to diagnose. It is an
artifact-producing diagnostic rather than a release-closure prerequisite, its
publication condition is always false, and candidate code receives no release
signing/notarization secrets.
The shortest authoritative path is therefore a matching stable tag -> one
formal build. A formal run refuses a tag whose target is not the frozen SHA,
an existing draft or published GitHub Release, identity mismatch, partial
signer configuration, incomplete native evidence, or asset drift. Operators
may force-update `vX.Y.Z` when no GitHub Release exists for that tag.

Platform acceptance is successful build and packaging on each matching native
runner. Windows additionally requires strict unsigned/signing proof and the
fresh formal sealing boundary before exact-asset verification. When the tagged
commit carries `docs/release/windows-formal-offline-admission.json` for that
version, formal Windows instead re-verifies the pinned offline-sealed
installers and fails closed on any mismatch; it does not publish an unsigned
fallback. macOS
additionally requires Developer ID signing, one Apple notarization of the
signed DMG, a stapled DMG ticket, and a stapled ticket on the original app.
The published macOS installer is that DMG, with `FyAgent.app` and an
`Applications` symlink so Finder can drag-install. The notarization helper
submits the DMG once, then polls `notarytool info`
until Apple returns a terminal status, instead of treating a
`notarytool wait` timeout as rejection. The Release workflow does not launch the setup executables. The separate
Windows installer lifecycle workflow runs the silent install and
uninstall diagnostic on fresh matching Windows runners.

## Windows acceptance and signed rehearsal

1. Run **Release** on protected `main` with mode `preflight` and the full
   candidate SHA. The native x64 and ARM64 jobs produce raw installers and
   build metadata. Keep the run ID, attempt, and source SHA together.
2. Run **Windows installer lifecycle** on `main` with that run ID, attempt,
   and source SHA. Each native Windows job verifies its input archive and
   source metadata, then records the silent lifecycle evidence. Use an
   independent Windows client VM to test SmartScreen, UAC, version upgrades, and the first
   interactive launch. Record each result against the installer SHA-256.
3. Once the protected Certum signer is ready, run **Release** on protected
   `main` with mode `signed-preflight` and that exact branch tip SHA. Both
   signed installers must pass independent verification on matching GitHub-hosted
   Windows runners, including SignTool chain and timestamp checks. This mode
   produces workflow artifacts without publishing a GitHub Release.
4. Use the formal tag workflow with a configured provider or an admitted,
   independently sealed installer pair. The release requires valid signatures
   for both Windows architectures. An authorized operator can start the
   release from any machine. The signing job selects a protected Windows x64
   runner by labels and uses that runner's activated SimplySign session.
   See [Windows release identity and recovery](../../../release/release-identity.md)
   for provisioning a replacement signer and verifying it before failover.

Release metadata retains real schema identities for download manifests,
platform builds, aggregate build metadata, and Windows signing status. The
Windows signing table in public notes is generated from verified metadata;
credentials and provider commands never become documentation or attachments.

Optional task archive or journal bookkeeping is outside the Release trust
chain. It does not create release eligibility, move a release tag, or replace
the tagged source SHA and workflow evidence.

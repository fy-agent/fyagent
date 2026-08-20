# Repository scan research

## Verified scope

The planning inventory was fixed to
`origin/main@9be29455a081d3ff0bc761465672727d09ffb3e6`. Candidate scans must be
rerun after all edits and recorded without printing secret values. Current-tree
and reachable-history results are separate claims.

## Existing capabilities

- No gitleaks, TruffleHog, detect-secrets, git-secrets, secretlint, GitGuardian,
  or equivalent repository-owned scanner/configuration is present in the
  current tree or CI workflows.
- GitHub's live `security_and_analysis` state on 2026-08-13 reports secret
  scanning, non-provider patterns, validity checks, and push protection as
  disabled. The alerts endpoint returns a disabled-feature response, so zero
  alerts cannot be claimed.
- `.gitattributes` assigns future
  `tests/e2e/visual-baselines/**/*.png` files to Git LFS. The current baseline
  directory contains only its README and manifest; CI verifies the attribute
  text but does not install, fetch, or validate LFS objects.
- The repository has narrow non-empty and maximum-size checks for release
  inputs, WebView2 fixtures, icons, logs, and visual assets, but no generic
  tracked-file or history-blob size gate.

## Planning inventory results

- 1,732 tracked files were enumerated from the baseline tree.
- No current tracked blob is at least 10 MiB.
- No reachable historical blob is at least 10 MiB.
- The three largest current blobs are reviewed marketing sample PNG files at
  approximately 1.72 MB, 1.71 MB, and 1.55 MB. The next is the macOS icon at
  approximately 1.33 MB. These are ordinary reviewed visual assets, not an
  unexpected large-object finding.
- The public Markdown path scan finds six real Windows profile-path lines and
  the known placeholder/demo examples described in `docs-contract.md`.

## Safe implementation boundary

Issue #21 asks for scan results and evidence, not a new dependency or a broad
security product. This task should:

1. Add a maintained, dated governance audit under `docs/fyagent/audits/` with
   the exact baseline SHA, candidate scope, commands, tool/source limitations,
   sanitized result counts, and reviewed exception categories.
2. Add a dependency-free repository audit helper that enumerates unique blobs,
   reads bytes only in memory, and reports only safe path/category/OID/count.
   It must cover explicit current trees and every blob reachable from all refs,
   including deleted, renamed, pathless, text, and binary blobs. Do not include
   values or matching lines in stdout, stderr, errors, task artifacts, the
   audit, the PR, or logs.
3. Treat any plausible live secret as a stop condition for private rotation and
   removal; a public “finding” must never quote the candidate value.
4. Record that GitHub secret scanning is disabled rather than implying that an
   unavailable alerts endpoint proves zero secrets.
5. Inventory current and reachable-history blob sizes with object IDs, byte
   counts, and safe paths. Use 10 MiB as an audit review threshold only, not a
   new CI limit. Record reviewed top blobs even when the threshold count is
   zero.
6. Avoid history rewriting and avoid adding an unreviewed generic size gate or
   secret-scanning dependency in this task.

## Reproducible helper contract

- Tracked identity/path scope: NUL-delimited `git ls-files` plus path-only
  pattern classification.
- Current-tree/history secrets: the repository-owned helper enumerates object
  IDs and obtains blob bytes through captured Git subprocess streams. Raw blob
  bytes are never forwarded to inherited stdout/stderr. A runtime-constructed
  synthetic fixture must prove a candidate value is absent from every
  serialized finding, error, stdout, and stderr surface.
- Current sizes: `git ls-tree -r -l <sha>`.
- History sizes: `git rev-list --objects --all` followed by
  `git cat-file --batch-check` and size/path aggregation.
- Live GitHub capability: repository `security_and_analysis` through the GitHub
  REST API; record the timestamp and feature state.
- Record exact helper, Git, gh, Node, mise, and REST API versions with the
  candidate audit.

The implementation must preserve raw secret candidates out of committed and
conversation-visible output. Only sanitized evidence is reviewable.

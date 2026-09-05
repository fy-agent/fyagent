# Security Review and Dependency Evidence

## 1. Scope / Trigger

Read before dependency remediation, security-alert triage, credential-example
changes, authentication/proxy logging, or native unsafe-boundary changes.
This contract governs evidence and review; domain owners still govern IPC,
secrets, installers, and release authority.

## 2. Signatures and Authorities

- Dependency authority: `package.json` / `pnpm-lock.yaml` and
  `src-tauri/Cargo.toml` / `src-tauri/Cargo.lock`.
- GitHub alert records are keyed by repository, alert number, scanned ref/SHA,
  rule/advisory and location. They are not evidence about a different local HEAD.
- Independent checks: pnpm audit, cargo-audit, Gitleaks and the maintained
  dependency-cruiser configuration. Record actual tool versions, advisory DB
  revision, input scope and exit status in task research/review artifacts.
- `proxy/providers/{copilot_auth,codex_oauth_auth}.rs` and
  `proxy/handler_context.rs` own their diagnostic call sites.
- `services/session_usage_{opencode,gemini,grokbuild}.rs` keep session/request
  IDs in their persistence and explicit sync-result boundary, not the reviewed
  ordinary log messages. Fixed failure events must not interpolate the raw
  returned error string. This does not change row identity, retry, or cost rules.
- Renderer/build input safety is owned by
  [Frontend Security Boundaries](../frontend/security-boundaries.md).

## 3. Contracts

### Dependency remediation

Trace each advisory through the lock graph to a real owner. Prefer compatible
updates or removal of an unused direct dependency. Do not force an incompatible
major version into a transitive API merely to eliminate a scanner match.
Check the complete lockfile, including build/dev and other-platform entries;
then explain actual feature/platform reachability separately.

Classify vulnerability, unsoundness, unmaintained and yanked reports separately.
An empty vulnerability list does not erase informational unsoundness warnings.
No broad ignore list, severity filter, registry-error suppression, test removal,
or security-switch disablement may be used to manufacture a clean result.
Dependency changes must pass locked build/type/behavior gates; test-runner
migration may require explicit fixture settings or a bounded timeout for a
real subprocess integration test, never weaker security assertions.

### Secret handling and alert triage

Inspect location/metadata before secret contents. Secret scans and saved tool
reports must redact values. Persist only the rule, path, line, classification,
reason and required action; never commit secret values or value-derived hashes.
Check encoded examples as well as plaintext and current trees as well as history.

A realistic key inside an example is not proven fake. Remove current copies,
substitute a visibly non-secret placeholder, and identify the required owner
revocation/rotation step. Source deletion is not revocation or history cleanup.
Do not test the credential against a live service or revoke an upstream client
without the required authority. Public installed-app OAuth client constants
must be distinguished from private user access/refresh tokens using primary
upstream evidence; do not rename or encode constants to evade scanning.

### Diagnostics and native ownership

Prefer fixed operation names, closed reasons and counts over account/session
identifiers, enterprise hostnames or raw vendor error bodies. Changing log level
is not redaction. Do not substitute stable secret-derived identifiers for raw
secrets. Existing broader diagnostic debt must be reported, not retroactively
described as fixed by a change to a few call sites.

Native pointer review traces allocation lifetime, API result, null checks,
common-header size and variable-length record bounds before casting. Preserve
reviewed native APIs and permission masks. A zero-initialized output buffer
filled by a checked OS CSPRNG is not a hardcoded nonce. Source-order tests prove
reviewed ordering only; they do not replace matching-host native tests.

Release cache and trust policy belongs to
[GitHub Release Workflow](./github-release-workflow.md). Removing a cache
consumer is mitigation, not proof that default-branch execution of a candidate
commit is isolated from other workflows.

## 4. Validation & Error Matrix

| Condition                                                             | Required result                                                                     |
| --------------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| Alert references another commit                                       | Compare current source/lock; do not claim it applies or is closed without evidence. |
| Scanner fails, registry/DB is unavailable, or scope misses TypeScript | Report incomplete evidence; never interpret it as zero findings.                    |
| Compatible patched dependency exists                                  | Update owner/lock and run affected gates.                                           |
| Only an incompatible upstream repair exists                           | Record exact chain, affected API/features and residual risk; no forced override.    |
| Candidate secret appears in docs/tests                                | Investigate origin; realistic values require conservative handling.                 |
| Current key copies are removed but history remains                    | Report revocation/rotation outstanding until separately verified.                   |
| Native pointer warning refers to borrowed OS storage                  | Prove owner lifetime and bounds; do not dismiss by rule name alone.                 |

## 5. Good / Base / Bad Cases

- Good: remove an unused XML dependency while upgrading the plist owner that
  still needs XML; prove no old affected parser remains in the lock graph.
- Base: an upstream maintenance warning remains and is explicitly scoped to
  its real dependency/feature chain with no false clean-security claim.
- Bad: dismiss every test-directory match, log a token hash, replace native
  crypto, or declare remote alerts closed after only a local commit.

## 6. Tests Required

- Re-run whole lockfile audit after remediation; preserve warnings and failure
  exit statuses in the evidence summary.
- Run redacted current-tree and history secret scans with explicit input scope.
- Run normal frontend/V2/Rust/build/browser/prearchive gates for touched owners.
- `tests/architecture/nativeSecurityOrdering.test.ts` requires the host ACE
  common header and minimum SID-size guard before the allowed-ACE reference.
  It also rejects the specific retired account/session diagnostic patterns;
  these source checks are not a replacement for whole-program taint analysis.
- Credential examples must round-trip through their actual encoding and retain
  explicit placeholders (`tests/deeplinkPlayground.test.ts`).

## 7. Wrong vs Correct

Wrong: `audit succeeded -> no security debt`; `delete key -> key revoked`.

Correct: report scanner categories separately, attach actual scope and revision,
fix validated issues, and name the remaining upstream/owner/native actions.

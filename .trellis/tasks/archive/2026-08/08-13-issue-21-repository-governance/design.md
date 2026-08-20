# Design: Issue 21 repository governance

## 1. Delivery shape

Use one task, one branch, and one focused pull request. The Issue #21 path
redaction and its same-day governance addendum share a single observable
outcome: the public repository must tell the truth about its source,
architecture, validation, review policy, and scan state without exposing local
workstation metadata.

No runtime code, public API, persistence, dependency, build topology, or
Release workflow changes are required. The implementation has four boundaries:

```text
tracked public content
  ├─ six privacy redactions
  ├─ three synchronized READMEs
  ├─ bilingual contributor and CODEOWNERS policy
  └─ dated sanitized governance audit
                  │
                  ▼
existing repository contract test
  + dependency-free sanitized audit helper
                  │
                  ▼
PR exact-head CI / Required
                  │
                  ▼
live main protection -> squash merge -> main push CI -> issue/task closure
```

## 2. Public documentation boundary

### 2.1 Privacy redaction

Use stable semantic placeholders rather than machine-root placeholders:

| Evidence role | Placeholder |
| --- | --- |
| VibeKey business-plan draft | `<local-submission-draft>` |
| VibeKey archive | `<local-vibekey-project-archive>` |
| VibeKey driver checkout | `<local-vibekey-driver>` |
| FyAgent source image | `<local-fyagent-source-image>` |

Only the path token changes. Preserve hashes, artifact roles, Git status,
dimensions, color-mode/alpha claims, and audit conclusions byte-for-byte where
practical. The archived Trellis line is a one-time privacy correction required
by the public-tree acceptance criterion; no other historical task text changes.

### 2.2 README synchronization

The three root READMEs remain language peers. Add the same semantic blocks in
each language rather than mechanically translating headings only:

- current configuration scope, with WorkBuddy identified as an independent
  entry rather than silently omitted from an apparently exhaustive provider
  list;
- concise architecture: React/Vite renderer -> Tauri IPC -> Rust
  commands/services -> SQLite, local target-tool configuration, and local
  proxy;
- maintained developer link;
- first checkout: mise version plus trust/bootstrap/system-check/dev;
- separate optional current-host build from the interactive startup flow;
- evidence ladder: current-host check, exact PR-head `CI / Required`, formal
  Release chain, and explicit non-HIL/non-signing limits.

Do not add version numbers, completion percentages, new distribution channels,
or security guarantees that will drift or exceed executable evidence.

### 2.3 Contribution topology

Keep `CONTRIBUTING.md` bilingual and explain remotes by role, not by assuming
one local configuration:

- canonical source is `fy-agent/fyagent`;
- maintainers may have canonical `origin`;
- external contributors normally use a personal fork as `origin` and the
  canonical FyAgent repository as an additional fetch source;
- CC Switch upstream synchronization is a separate fetch-only maintenance
  contract and is not the contributor's canonical FyAgent remote.

The merge path is a branch based on current canonical `main`, a focused PR,
successful exact-head `CI / Required`, then squash merge. The documentation
must not promise that arbitrary branches are deleted or that human approval is
required.

Keep the CODEOWNERS mappings, but state that they route ownership and become a
merge gate only if GitHub protection explicitly requires Code Owner review.

## 3. Executable documentation contract

Extend `tests/currentDocsContract.test.ts`, already part of Repository
Contracts, instead of creating another runner.

Add a tracked-Markdown enumerator backed by NUL-delimited `git ls-files` so the
test covers current docs, historical docs, and committed Trellis archives
without reading untracked local files. For every tracked Markdown file:

1. locate case-insensitive Windows `Users` path segments;
2. allow only a following angle-bracket placeholder for user-profile examples;
3. reject concrete profile names and report only the file as the assertion
   label;
4. keep explicit positive fixtures for localized placeholder and demo examples.

Do not broadly reject every absolute path. System roots, test roots, URL
schemes, registry language, and non-profile examples are outside this privacy
rule.

Extend the existing README/public repository assertions to pin the new
architecture, onboarding, evidence, canonical-source, remote-role, merge-flow,
and advisory-CODEOWNERS semantics. Tests should assert durable concepts rather
than full translated paragraphs.

## 4. Governance audit evidence

Add one maintained dated audit under `docs/fyagent/audits/`. It records:

- Issue URL, audit date, baseline SHA, candidate/PR scope, and repository;
- current-tree and reachable-history scan boundaries;
- exact reproducible command families and relevant tool/API state;
- sanitized counts/categories for workstation paths, accounts/local IDs,
  high-confidence secret shapes, and blobs;
- reviewed legitimate attribution/owner/example/asset categories;
- GitHub secret-scanning and push-protection state as observed, without
  claiming an unavailable alert count;
- LFS coverage and the absence of a generic size gate;
- no-history-rewrite and no-runtime/HIL limitations.

The audit must never contain a candidate secret. Scans emit path/category/count
only. A plausible live secret stops publication until private remediation is
authorized. The 10 MiB blob value is an audit review threshold, not a new
repository limit.

Add `scripts/audit/repository-governance-scan.mjs` as a bounded audit helper,
not as a new default CI secret-scanning product. It must:

- accept an explicit treeish for candidate/current-tree scanning;
- enumerate unique blobs reachable from explicit refs for history scanning,
  including deleted, renamed, pathless, text, and binary blobs;
- read blob bytes only inside the process and classify high-confidence secret
  shapes without echoing matching lines or byte slices;
- sanitize a path if the path itself matches a protected shape;
- emit JSON containing only scanner version, source/tree identity, category,
  safe path, object ID, counts, size inventory, and failures;
- fail closed on Git/object/parser errors or incomplete enumeration;
- have synthetic executable tests that construct a candidate at runtime and
  assert it is absent from stdout, stderr, serialized findings, and thrown
  errors.

Capture exact versions for the helper, Git, GitHub CLI, Node, mise, and the
GitHub REST API version used. Do not claim gitleaks or GitHub Secret Scanning
coverage when those tools/features did not execute.

The audit is fixed to a verified baseline and the PR candidate; it must not
embed a self-referential final commit SHA. The PR provides the exact final head
and CI evidence, and the merge provides the final `main` SHA.

## 5. Live GitHub policy

### 5.1 Approved target

`main` must retain its current policy and add only the GitHub Actions-owned
aggregate observed during planning (`app_id: 15368`):

```json
{
  "required_status_checks": {
    "strict": false,
    "checks": [{ "context": "CI / Required", "app_id": 15368 }]
  }
}
```

Approvals remain zero; Code Owner and last-push approval remain false. Required
PRs, administrator enforcement, and force-push/deletion prohibitions remain
true/true/false/false respectively. No ruleset, linear-history,
conversation-resolution, branch-lock, or fork-sync behavior changes.

### 5.2 Optimistic compare/update operation

GitHub documents conditional ETag requests for GET but does not document
conditional `PUT` for this endpoint. Therefore this operation is not an atomic
compare-and-set and must not be described as one. Because required status
protection is absent, use GitHub's full protected-branch update only after all
of these hold:

1. exact PR head is known;
2. its `CI / Required` check is completed successfully;
3. a fresh protection GET is normalized into the writable request schema and
   equals the expected pre-change policy;
4. the full outbound payload has been reviewed and contains every existing
   writable setting explicitly;
5. the GET-to-PUT interval is kept minimal and no parallel mutation is known.

After update, perform a new GET and exact-field comparison. Verify that the
required check is bound to GitHub Actions app ID `15368`. If the response
differs, fetch once more before any rollback. Restore the normalized pre-change
payload with `required_status_checks: null` only when that fresh state exactly
equals this task's expected post-update state; that proves no third party has
changed it since this write. If it differs in any other way, do not overwrite
it again: stop, report the non-atomic race, and require human reconciliation.

## 6. Validation and publication lifecycle

### Local and static

- focused public documentation contract;
- formatting and link/doc contracts reached by repository checks;
- canonical `mise run check` on this Windows host;
- full diff, path scan, sanitized secret scan, object-size inventory, and
  public-language review;
- separate read-only code-quality, docs-quality, and security/diff review.

Local results prove only the executed current host and static contracts.

### Remote

1. Re-fetch canonical `main`; stop if the branch cannot be cleanly rebased or
   if the source requirement changed materially.
2. Complete every intended file, stage the exact candidate, derive its index
   tree through `git write-tree`, and run all content/secret/size scans against
   that tree. Any later file change invalidates and repeats the affected scans.
3. Commit and verify the commit tree equals the scanned index tree; run a
   read-only exact-head scan consistency check whose result is recorded in the
   PR/session rather than creating a self-referential commit.
4. Push the task branch and create a non-draft PR with `Closes #21`, exact
   tests, risks, external policy change, and rollback.
5. Bind remote evidence to the PR number, exact head SHA,
   `.github/workflows/ci.yml`, event `pull_request`, latest run attempt, and
   GitHub Actions app ID `15368`. Wait for exact-head `CI / Required`; do not
   infer success from component jobs or an older SHA.
6. Apply and verify branch protection.
7. Reconfirm mergeability and exact-head check, then squash merge.
8. Read `merge_commit_sha` from the merged PR, verify it is reachable from
   remote `main` (it need not remain the tip if another merge races), confirm
   Issue closure, and wait for that exact SHA's latest-attempt `push`
   `CI / Required` from `.github/workflows/ci.yml` to succeed.
9. Verify current-tree redaction and live protection from fresh remote state.

## 7. Rollback and stopping conditions

- Before merge: amend or close the PR; the task branch isolates repository
  content. Restore branch protection from the verified pre-change snapshot if
  its mutation was attempted.
- After merge: use a normal revert PR. Never force-push or delete `main`.
- Stop immediately on a plausible live secret, an unexpected protection drift,
  a failed/cancelled/stale/absent Required check, unresolved merge conflict,
  material new Issue requirement, or ambiguous merge SHA.
- Task archival happens only after the full remote acceptance loop. Run local
  finish/archive with `--no-commit`; do not push the resulting administrative
  archive/journal diff or claim remote `main` contains the archived task. A
  future public administrative update requires separate authorization.

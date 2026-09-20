# First-use Software Recommendations

## 1. Scope / Trigger

Read before changing first-use presentation on `/agents`. `FirstUseGuide.tsx`
owns the two-step UI; `firstUseRecommendations.ts` owns the route-local purpose
association. [Native First-use State](../backend/first-use-guide.md) owns device
eligibility and persistence. [Agent Directory](./agent-directory.md) still owns
catalog, installation, configuration and software order.

## 2. Signatures

```ts
type GuidePurpose = "office" | "coding" | "both";
type FirstUseGuideState = "pending" | "dismissed";
SettingsPort.getFirstUseGuideState(): Promise<FirstUseGuideState>;
SettingsPort.dismissFirstUseGuide(): Promise<"dismissed">;
```

Use `featureKeys.firstUseGuide` through the shared query owner. The native
adapter parses unknown responses; browser fallback returns dismissed and
rejects native-only dismissal instead of pretending to save.

## 3. Contracts

### Route and startup

- Only the directory branch may show onboarding. A valid explicit `target`
  remains authoritative and disables the first-use query. Query activity follows
  persistent route visibility.
- Wait for catalog and guide-state settlement before presenting the directory
  or guide and acknowledging frontend-ready. Do not use RAF/document visibility
  for native startup readiness. A failed guide read opens the ordinary directory,
  not an invented fresh-install state.
- A catalog error, empty catalog or target/catalog mismatch keeps the existing
  directory error/empty/recovery surface and its ready acknowledgement. Do not
  mount an incomplete guide or write dismissal without a valid parsed catalog.
- Lazy-load the one-time guide and its CSS only for a pending user. The committed
  guide component owns frontend-ready while its chunk is loading; a Suspense
  placeholder must not reveal the native window. Catalog error/empty states keep
  their own ready acknowledgement instead of waiting for an unmounted guide.
  Keep the existing initial-JavaScript budget; do not raise it to admit onboarding.
  Audit the static dependency closure as well as page chunks: a hook first used
  in a lazy page can still grow a bootstrap-shared vendor chunk. Keep this narrow
  write consistent with the existing guarded local-operation pattern.

### Recommendation projection

- The question is one sentence, with office, coding and combined-use options
  and a skip action. Selection shows a small set of recommendations, with
  back/reselect and a full-directory action.
- Office recommends QoderWork CN, TRAE Work CN and WorkBuddy; coding recommends
  Grok Build, Codex, Claude Code and OpenCode; combined use recommends WorkBuddy
  and Codex.
  These are purpose associations, not capability, platform or quality rankings.
  Intersect IDs with the parsed catalog and preserve its names/order. Do not
  create a fallback catalog or infer installation/action permissions.
  For the current seven products, the office/coding union covers the complete
  catalog, with a concise purpose description for each. The purpose-description
  map is exhaustive over the closed `AgentCatalogId`; never fall back to a
  generic catalog description when a reason is missing. Cross-check identities
  against `AGENT_CATALOG_IDS`; do not silently omit a product by testing only a
  copied shortlist. A future new identity must receive an explicit description,
  while any deliberate exclusion from both single-purpose sets requires a
  recorded product rationale. Combined use remains a curated entry point, not
  the full catalog union.
- Purpose is component-local. Choosing it performs no native write, auth,
  install, model change or telemetry.

### Completion and lifecycle

- Directory scanning starts only after the first-use check settles with no guide
  or after dismissal succeeds. Never start it behind the guide.
- Completion and skip use the same narrow native dismissal. Keep the current
  step on failure with a safe retry message; never display raw native errors.
  A synchronous admission guard and disabled controls prevent duplicate writes.
- Cache only successful persisted dismissal. Hidden/unmounted completion may
  update the shared state but must not steal focus, reopen portals or scan.
  Focus the guide heading on step changes and the directory heading on visible
  completion. Reuse Button/PressableButton, brand assets and semantic CSS tokens.
  The common Agent page owns brand-size tokens; recommendations use a 64px
  frame with 48px artwork.

## 4. Validation & Error Matrix

| Condition                                             | UI behavior                                                         |
| ----------------------------------------------------- | ------------------------------------------------------------------- |
| Pending and valid catalog on directory                | Show purpose question, not a directory flash.                       |
| Dismissed / old installation                          | Ordinary directory.                                                 |
| Explicit software target                              | Existing configuration view, no guide interception.                 |
| Guide read fails                                      | Ordinary directory; no acknowledgement write.                       |
| Catalog fails, is empty, or cannot resolve the target | Existing catalog error/empty/recovery UI; no guide acknowledgement. |
| Save pending                                          | Keep guide; disable duplicate actions and choices.                  |
| Save fails                                            | Keep current recommendations; allow retry.                          |
| Save succeeds                                         | Full directory; no repeat with a fresh query client.                |
| Route hidden while save finishes                      | Update cache without focus or scan work.                            |

## 5. Good / Base / Bad Cases

Good: recommend current catalog names, then let the user browse all software.
Base: restart an unfinished guide at the purpose question. Bad: turn a purpose
selection into an automatic installation, persist a marketing profile or force
existing installations through onboarding after an upgrade.

## 6. Tests Required

`pages/agents/Page.test.tsx` covers all three sets, reselection, skip,
completion, new query-client restart, delayed/failed persistence, duplicate
clicks, unknown startup reads, target links and hidden completion. Native-port
tests reject unknown states and pending write acknowledgements.
The office/coding coverage assertion compares recommendation identities with
the shared catalog IDs, independently of per-choice expected names. Typecheck
must reject a new closed catalog identity without an explicit purpose reason;
tests also reject using the generic entry description as that reason. Keep Grok
Build's coding reason, supplied catalog order and current-name regression.
`browser/first-use-guide.spec.ts` covers keyboard focus, both saved themes,
loaded brand artwork at the shared dimensions,
large-small-large viewport changes, real click reachability, persistence and
positive catalog copy in Chromium/WebKit. Run the existing directory/browser
regressions and production boot gate. WebKit keyboard tests use its Option-Tab
all-controls navigation, without changing the host keyboard-access setting.
The four-item coding result must keep recommendations and completion
controls reachable at the smallest supported viewport. Browser fixtures do not
prove native first-install or Windows/macOS installer behavior.
The production navigation smoke test also verifies that a fresh user loads the
guide chunk and a post-skip reload does not request it.

## 7. Wrong vs Correct

Wrong: `if (!localStorage.getItem("welcome")) showGuide()`, using
`reason ?? entry.description`, or hiding unsupported actions by rewriting their
runtime status. Correct: native first-use state owns eligibility; every closed
catalog identity has an explicit purpose reason; only catalog summary prose is
positive-only, while actual operation errors, unknown states and security
confirmations stay visible.

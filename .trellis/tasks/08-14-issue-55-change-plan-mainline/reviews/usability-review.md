# Issue #55 prototype usability review

## Round 1

Result: `USABILITY_REVIEW=FAIL` (`0 P0 / 4 P1 / 3 P2`).

This was an independent static usability review of the frozen `prd.md` and
`process-state-machine.md` UI contract, `design-freeze.md`, the generated visual
reference, the high-fidelity HTML prototype, and its manifest. Visual continuity
was also compared with the current FyAgent V2 Prompt/Memory surfaces in
`src/App.tsx`, `src/components/prompts/PromptPanel.tsx`,
`src/components/prompts/PromptListItem.tsx`, and
`src/components/hermes/HermesMemoryPanel.tsx`.

The authority reviewed is DESIGN_FREEZE commit
`d158b27690d897e8e9f2ece7d8887da6423b899c`. Evidence classes are
`prototype` and `code_audit` only. Opening the prototype through `file://` is
blocked by the browser safety policy; this review did not start a server or run
a browser, test, build, renderer, or native command. It is not
`runtime_screenshot`, `native_runtime`, `failure_path`, `UAT`, production
evidence, or proof of the final 1440x960 rendered result.

### Exact byte identity

| Artifact | SHA-256 | Other identity |
| --- | --- | --- |
| `research/prototype/generated-change-plan-reference.png` | `d8bd36f8a31babaed4bc50f33f3c8e2c593c3b440cd6578a296057f370095681` | 1,275,699 bytes; 1536x1024; inspected with `view_image` |
| `research/prototype/change-plan-prototype.html` | `2ceb3a94b1e47d2673d243abaa59e290fee6d666edd4694eb1973b9e0cd08514` | 61,366 bytes; manifest viewport 1440x960 |
| `research/prototype/manifest.json` | `782efc3c2ba28d48eafd7b8fbcd4b75cee1426e3f20c7f4edfe4da818d3ee8af` | 1,042 bytes |

The image and HTML hashes, sizes, image dimensions, DESIGN_FREEZE SHA, and
evidence label match the manifest. The manifest correctly states that both
artifacts are prototypes and not runtime/UAT evidence.

### Findings

| Severity | Finding | Required closure |
| --- | --- | --- |
| P1 | The generated reference contradicts three frozen safety semantics even though its visual hierarchy is useful. It shows a Provider “connection/capability verification” step, includes a proxy resource/action, and promises automatic rollback/backup restoration. The frozen contract forbids outbound Provider probing/model traffic during preview/apply, excludes proxy route/takeover from this slice, and promises readback classification plus manual recovery hints rather than automatic inverse/restore. Merely labelling the image `prototype` does not prevent these cues from steering implementation or usability judgment in the wrong direction. | Regenerate the reference with the same restrained FyAgent visual language but remove Provider connectivity/capability probing, proxy effects, and automatic rollback/restore claims. Show only declared affected resources, conditional backup as the first durable effect, independent readback, and `manual_required` limits. Update and re-read the image/manifest hashes after replacement. |
| P1 | The operation fixture is not complete enough to validate the first-slice semantics. The only create case is `create_and_select` (`change-plan-prototype.html:1510`), so the prototype never demonstrates `create_only` with no current/live/common/MCP effects. The only edit case is `current_provider` (`:1581`), so it never demonstrates a non-current edit whose affected-resource set excludes live routing. A generic selector label of “create”/“edit” hides these materially different user outcomes. | Add separately named fixtures and controls for `create_only`, `create_and_select`, `edit_current`, `edit_non_current`, and `switch`. For every fixture, make intent, before/after route, resources, ordered actions, backup/recovery scope, and secretRef status match the frozen operation semantics. Keep no-op outcomes out of the executable Plan fixtures. |
| P1 | The `unsupported` projection has no Plan ID/digest (`:1917`) but leaves the prior operation’s Plan-only summary, resources, ordered actions, recovery copy, “来自已保存、不可变的 Plan snapshot” claim (`:1309`), and one-confirmation panel visible. This presents a non-executable planning outcome as if an immutable Plan exists and can leave stale semantics on screen after switching state. The two near-identical return/modify actions also weaken the single safe exit. | Give unsupported its own non-executable layout. Remove or explicitly replace Plan-only snapshot/resources/actions/binding content, show the exact unsupported reason and zero-write/no-fallback boundary, and expose one unambiguous return/modify action. State transitions must not retain stale Plan semantics from the preceding executable state. |
| P1 | Safety-state accessibility is not closed. The alert keeps the same visual checkmark for clean, expired, drift, unsupported, and secret-missing (`:1402-1406`), so invalid states communicate success by icon. The script changes `role` to `alert` (`:1893`) while the node retains `aria-live="polite"`, creating conflicting announcement urgency. The prototype also does not define initial heading focus on route entry or a stable focus/announcement rule when a ready Plan becomes invalid. For a confirmation surface, loss of authorization must not rely on color and text discovery alone. | Use state-specific, non-color status icons and labels; use one coherent live-region contract (`status` + polite for ordinary updates, `alert`/assertive for newly invalidated or blocked states); announce code and reason once; preserve the user’s current control focus during live updates; and define initial focus on the page heading plus deterministic focus to the safe recovery CTA only when navigation creates a new blocked screen. Keep invalid-state confirmation disabled. |
| P2 | Contract-critical text is repeatedly rendered at 9–10px (`:370`, `:401`, `:500`, `:577`, `:597`, `:627`, `:761`, `:776`, `:814`, `:833`, `:842`, `:850`, `:872`). This includes resource keys, baseline/readback details, state codes, secretRef/privacy explanations, and binding metadata. At the preferred 1440x960 viewport the overall two-column hierarchy is compact and plausible, but these sizes are below comfortable scan size for a high-stakes confirmation screen; actual clipping/scroll behavior is unproven because browser rendering was blocked. | Raise safety- and contract-critical copy to at least 12px with adequate line height, keep body copy at a comfortable 13–14px, and reclaim height through spacing/disclosure rather than miniature text. After runtime is permitted, capture fresh 1440x960 plus narrow-width evidence and verify no footer occlusion, truncation of reasons, or horizontal scroll. |
| P2 | FyAgent V2 continuity is only partial. The neutral/blue palette, restrained cards, rounded controls, and compact header align reasonably with Prompt/Memory, but the prototype hard-codes `color-scheme: light` (`:6`) and standalone hex tokens. Current Prompt/Memory/App surfaces use shared semantic tokens and explicit dark variants (for example `PromptPanel.tsx:267`, `HermesMemoryPanel.tsx:84,107`, and `App.tsx:1444-1464`). In dark mode this prototype would look like a separate application rather than the next FyAgent V2 surface. | Map the prototype to FyAgent semantic surface/text/border/primary/warning/danger tokens and add a dark-mode fixture or system-theme rendering. Retain the current dense desktop information architecture, but demonstrate that state tones, disabled confirmation, focus rings, and disclosure rows remain legible in both themes. |
| P2 | The frozen product requires complete `zh`, `zh-TW`, `en`, and `ja` projections, while this prototype hard-codes only `zh-CN`. As a design artifact it does not need to be production-i18n code, but the current bytes cannot expose longer English/Japanese wrapping, CTA ambiguity, or locale-specific truncation in the 354px safety column and sticky footer. | Add a prototype locale fixture (or four static content fixtures) for the six reviewed states and five operation variants. Keep the artifact explicitly non-runtime, but use it to verify hierarchy, CTA labels, state reason wrapping, and privacy/recovery copy at 1440x960 and narrow widths before implementation locks the layout. |

### Contract checks that passed statically

- `clean`, `warning`, `expired`, `drift`, `unsupported`, and
  `secret-missing` are independently selectable. Warning retains one
  confirmation; every invalid/blocked state disables confirmation and exposes a
  recovery/return CTA.
- The HTML directly answers what changes, affected managed resources, ordered
  actions, baseline/readback predicates, backup and manual-recovery limits,
  credential-reference status, privacy, expiry/invalidation, and exact
  `planId + planDigest` confirmation.
- Preview copy explicitly says it persists only the Plan snapshot/lifecycle
  metadata, does not change Provider/DB current/tray/cache/jobs/events/backups,
  and makes no Provider/model outbound request.
- Prototype controls are visibly labelled “Prototype only” and explicitly say
  they are not runtime UI and trigger no writes (`:1239-1259`). Button handlers
  only explain the intended contract; they do not claim runtime execution.
- Visible focus styles, native keyboard-operable `details/summary`, responsive
  breakpoints, reduced-motion handling, disabled confirmation, and live status
  text are present. The P1 accessibility finding above is about incorrect state
  signalling/focus semantics, not absence of all accessibility work.
- Static sentinel scanning found no absolute path or secret value in the HTML
  or manifest. The displayed `secretRef` strings are synthetic opaque
  identifiers and are explicitly labelled opaque; the copy says values are
  neither read, displayed, nor hashed.

### Round-1 closure checklist

1. Replace the generated reference so its semantics match the frozen contract.
2. Add both create variants and both edit-currentness variants.
3. Give unsupported a true non-Plan projection with no stale Plan content.
4. Correct invalid-state icons, live-region urgency, and focus rules.
5. Raise contract-critical typography and verify the 1440x960/narrow layouts
   only after browser evidence is authorized.
6. Align light/dark tokens with the current FyAgent V2 Prompt/Memory surfaces.
7. Add four-locale design fixtures for wrapping and CTA review.

`USABILITY_REVIEW_ROUND_1=FAIL`

## Revision 2 full re-review

Result: `USABILITY_REVIEW=FAIL` (`0 P0 / 3 P1 / 1 P2`).

This was a fresh full static review of the current bytes, not an acceptance of
the Revision-2 repair summary. It re-read the frozen PRD/process resource and
state contracts, inspected the replacement image with `view_image`, read the
complete HTML/locale/operation/state data and render functions, checked the
manifest binding, and replayed all seven Round-1 findings against the current
implementation.

The evidence boundary remains `prototype` plus `code_audit`. Browser `file://`
opening remains blocked by policy. No server, browser, test, build, renderer, or
native command was run, and this is not runtime, screenshot, failure-path, UAT,
or production evidence.

### Revision-2 exact byte identity

| Artifact | SHA-256 | Other identity |
| --- | --- | --- |
| `research/prototype/generated-change-plan-reference.png` | `4d47a2e353c5fcc1df7b1c6622134f1d1057fa5b4548c4b3b263d9953ab2b051` | 1,262,692 bytes; 1536x1024 |
| `research/prototype/change-plan-prototype.html` | `5112a618abb2c00f168749313fa757ee97be71f4db8894452846d32368a16c75` | 106,010 bytes; preferred viewport 1440x960 |
| `research/prototype/manifest.json` | `1a00cf43132f3beb0510a673ff3867391cc9605df4496beb4b2b54fffb3788d1` | 1,162 bytes |

The image/HTML hashes, sizes, image dimensions, DESIGN_FREEZE commit
`d158b27690d897e8e9f2ece7d8887da6423b899c`, freeze-manifest hash, evidence
class, and Revision-2 coverage note all match the manifest. A read-only syntax
parse reports valid JavaScript and 82 unique HTML IDs. An inert fixture audit
finds all four locales, five operation keys, six state keys, and 120 reachable
locale × operation × state combinations, with each localized `changes[]` count
matching its current resource array. Those structural results do not close the
semantic cross-product findings below.

### Remaining findings

| Severity | Finding | Required closure |
| --- | --- | --- |
| P1 | The five fixtures now distinguish activation/currentness correctly, but their displayed affected resources and apply order are still not the frozen Plan contract. `operationSpecs` currently exposes 1/5/4/1/5 resources for create-only/create-and-select/edit-current/edit-non-current/switch (`change-plan-prototype.html:1526-1574`). The frozen resource matrix (`design.md:337-345,375-400`) separately requires Provider row **and endpoint set** for create/edit; DB current and device current where selection changes; and separate Codex catalog, auth, and config resources with independent fingerprints/readbacks rather than one `codex.live_projection`. The HTML and replacement image omit these resources or collapse them into one fingerprint. Their three/four high-level cards likewise claim to be the reviewed ordered actions while hiding the exact cross-resource order. A user therefore still cannot verify every write/readback that the exact Plan authorizes. | Make each fixture derive its resource rows from the frozen mutation-resource matrix: separate Provider row/endpoints, DB/device current, catalog/auth/config, common, MCP, and conditional source backfill as applicable. Each row needs its own safe key, baseline, effect boundary, and readback. Render the actual ordered actions, or a grouping whose expanded children preserve every frozen action and exact order. Synchronize the generated switch reference and manifest to the same complete switch set. |
| P1 | The nominal 5×6 cross-product is reachable but not semantically valid. Every locale's global `warning` state says managed live configuration will change (`:1698`, `:1727`, `:1756`, `:1785`), directly contradicting `create_only` and `edit_non_current`, whose summaries state that live projections remain unchanged. Drift is also hard-coded to `codex.current · fingerprint_mismatch` for every operation rather than the operation's actual invalidated resource. More seriously, `renderOperation` writes an “available/version matches” credential status (`:1915`), while `renderState` never overrides or restores it; selecting `secret-missing` therefore shows “credential reference cannot be verified” in the alert and “metadata available/version matches” in the credential card at the same time. This is stale safety copy, not merely a localization nuance. | Define operation-aware warning and drift reasons (or a closed validity matrix that removes impossible combinations). Make `renderState` deterministically render every state-sensitive field, especially credential status, and restore the operation's clean/warning value when leaving a blocked state. All 120 combinations must have one internally consistent status, resource reason, credential status, risk, CTA, and confirmability result. |
| P1 | The narrow layout still has a deterministic structural failure despite the responsive media queries. `.intent-route` retains `minmax(150px, 1fr) 28px minmax(150px, 1fr)` plus two gaps (`:485`) at every width, while the declared minimum viewport is 320px and the mobile main/ribbon padding leaves substantially less than 344px. The before/after route must overflow horizontally. The primary intent title also remains `white-space: nowrap` with ellipsis (`:480`) and has no full-text disclosure, so longer English/Japanese operation titles are truncated on the same supported narrow surface. | At the mobile breakpoint, stack before/after route nodes or use true zero-minimum columns that fit the available inner width; allow the primary intent to wrap without covering the preview state. Keep critical text and both route endpoints fully available to sighted and assistive users. Browser/runtime evidence remains a later gate, but the CSS must first remove this statically guaranteed overflow/truncation. |
| P2 | Visible four-locale copy is complete, but several accessibility names remain hard-coded Chinese after locale switching: Plan metadata (`:1324`), prototype controls (`:1337`), before/after route (`:1373`), and the safety-summary aside (`:1432`). The generic prototype-controls `div` also has an `aria-label` without a grouping role. English, Japanese, and Traditional-Chinese screen-reader users therefore receive mixed-language regions even though visible text changes. | Add these accessible names to every locale fixture and update them during `renderLocale`; give the prototype controls a real `group`/`fieldset` semantic or remove its ineffective generic label. Re-audit every non-visible accessible name alongside visible locale copy. |

### Round-1 closures verified

- The replacement reference no longer contains Provider probe/capability-test,
  proxy-effect, automatic rollback, or automatic backup-restore semantics. It
  explicitly states no Provider outbound traffic and manual/readback recovery.
- Five executable operation fixtures exist: `create_only`,
  `create_and_select`, `edit_current`, `edit_non_current`, and `switch`. None is
  a no-op: absence, revision, or current target changes in every fixture.
- Unsupported is a distinct non-Plan projection. Plan metadata and Plan content
  are hidden from the rendered/accessibility tree; back, close, and confirm are
  hidden; exactly one return/modify action remains; the no-Plan/zero-write/no-
  legacy-fallback facts are explicit.
- All six states have distinct non-color symbols. Ready/warning uses a polite
  status; blocked Plan states use assertive alerts; the unsupported screen owns
  its separate assertive alert. Initial navigation focuses the page heading,
  state-control changes preserve focus, and the explicit
  `navigationCreatedBlockedScreen` path moves focus to the safe recovery CTA.
  Invalid Plan confirmation is disabled; unsupported has no confirmation.
- Static CSS inspection finds no font size below 12px. Body/recovery/status copy
  is generally 13–14px.
- Semantic FyAgent-like tokens drive light, dark, and system themes; state,
  border, surface, focus, and disabled styles no longer hard-code a light-only
  page.
- Four locale fixture objects contain all five operations and all six states,
  set the document language, and regenerate both selectors without losing the
  selected operation/state. The P1 cross-product and P2 accessible-name issues
  above are the remaining semantic/localization gaps.
- Static sentinel review found no absolute path, credential value, external
  URL, external script/style/image reference, fetch/XHR/WebSocket/beacon, or
  navigation side effect. Displayed secretRefs are synthetic opaque IDs and the
  image exposes only an opaque reference class, never a value.

### Round-2 closure checklist

1. Expand every operation and the reference image to the exact frozen resource
   and ordered-action set.
2. Make warning, drift, and credential status operation/state-aware for all
   120 combinations.
3. Remove the guaranteed narrow-width route overflow and intent truncation.
4. Localize all accessibility-only labels and give the prototype control group
   valid semantics.

`USABILITY_REVIEW_ROUND_2=FAIL`

## Revision 3 full re-review

Result: `USABILITY_REVIEW=PASS` (`0 P0 / 0 P1 / 0 P2`).

This was another independent full static review of the current bytes. It did
not rely on the main-thread closure summary. The review re-read the frozen
operation/resource/action matrix and UI state contract, inspected the current
PNG at original detail with `view_image`, read the complete HTML/CSS/JS and all
four locale fixtures, checked every Round-2 finding against the render paths,
and verified the manifest against the exact files.

Evidence classes are strictly `prototype` and `code_audit`. Browser `file://`
opening remains blocked by policy. No server, browser, test, build, renderer, or
native command was run. This PASS is not `runtime_screenshot`,
`native_runtime`, `failure_path`, `UAT`, production evidence, or proof of an
actual rendered interaction session.

### Revision-3 exact byte identity

| Artifact | SHA-256 | Other identity |
| --- | --- | --- |
| `research/prototype/generated-change-plan-reference.png` | `a70a2845e017405aaea9a12449e2b58c9b0a1023d0748fd655063c9c732d81ea` | 1,311,009 bytes; independently read as 1536x1024 |
| `research/prototype/change-plan-prototype.html` | `fe8f8cc0ac389c7c3cca9b3fd5005ff13d51f4cad8a1cb7d3d4a0221a9122dc4` | 124,238 bytes; preferred viewport 1440x960 |
| `research/prototype/manifest.json` | `d19ecec27e24904c57ceb6c232d24605029c70a35a8dc190f94ac1b5452a837a` | 1,311 bytes |

The manifest binds these exact hashes/sizes/dimensions, DESIGN_FREEZE commit
`d158b27690d897e8e9f2ece7d8887da6423b899c`, and freeze-manifest SHA
`2c1b753acf470699b428ae8a9eb401be82dbb0f1f19d9c13a6268350d4edfc5f`.
Its evidence-class and non-runtime statement are accurate.

### Round-2 closures verified

1. **Exact resources and actions — closed.** The operation specs now encode the
   frozen mutation matrix without collapsing independent fingerprints:

   - `create_only`: 2 resources; action `[2]`;
   - `create_and_select`: 10 resources; actions `[1,2,3,4,5,6,7,8,9]`;
   - `edit_current`: 7 resources; actions `[2,5,6,7,8,9]`;
   - `edit_non_current`: 2 resources; action `[2]`;
   - `switch`: 8 resources; actions `[1,3,4,5,6,7,8,9]`.

   Provider row and endpoint set are separate resource rows but correctly share
   the atomic action-2 card and render as its two children. DB current, device
   current, Codex catalog/auth/config, common config, managed MCP, and optional
   source backfill are separate where applicable. Every resource has a safe
   stable identity, baseline/version, effect boundary, and readback predicate
   (`change-plan-prototype.html:1547-1616`); localized change arrays and action
   arrays have exact matching cardinality. The replacement switch reference
   shows the same eight resources and stable action numbers 1 and 3–9, with no
   probe, proxy effect, automatic replay, or automatic rollback promise.

2. **Operation-aware 5×6 state semantics — closed.** Each operation now owns
   its warning and drift reasons. `deriveStateView` selects those reasons,
   substitutes a locale-safe missing/unverifiable credential status only for
   `secret-missing`, and restores the operation's normal opaque-reference status
   for every other state (`:1857-1895`). `renderState` writes all state-sensitive
   fields on every transition, while operation changes call both
   `renderOperation` and `renderState`; the prior stale credential/warning copy
   cannot survive. A read-only fixture audit found four locales × five
   operations × six states = 120 structurally reachable combinations, with no
   no-op executable fixture.

3. **Unsupported projection — remains closed.** Unsupported owns a separate
   assertive non-Plan projection. Plan metadata/content, back, close, and
   confirm are hidden from the rendered/accessibility tree; the screen states
   that no Plan/actions/recovery envelope exists, no managed write occurred,
   and no legacy fallback is used. Exactly one return/modify CTA remains. No
   stale operation Plan content is exposed.

4. **320px and long-locale layout — closed statically.** Intent and route text
   now wrap; their columns use true zero-minimum tracks. At 920px the ribbon is
   one column, and at 680px the before/after route stacks vertically with a
   rotated direction marker (`:1160-1257`). Resource keys and changes also wrap
   on the narrow surface, and action cards use auto-fit zero-overflow sizing.
   The former mathematically guaranteed 320px overflow and invisible full
   English/Japanese intent no longer exist. Actual screenshot/pixel behavior
   remains a later runtime evidence gate, not part of this static PASS.

5. **Localized accessibility names — closed.** Prototype controls are a real
   `role=group`. Plan metadata, controls, before/after route, safety aside, and
   back action have locale-owned accessible names for `zh-CN`, `zh-TW`, `en`,
   and `ja`; `renderLocale` updates each name together with the visible copy
   (`:1951-1970`). Initial heading focus, focus-preserving selector changes,
   explicit blocked-screen recovery focus, state-specific non-color symbols,
   polite ready/warning status, assertive blocked alerts, invalid confirmation
   disabling, and reduced-motion behavior remain intact.

### Full-rescan checks

- A read-only JavaScript syntax parse passed; all 85 HTML IDs are unique.
- A separate inert data audit matched the exact resource type/order and action
  arrays above; all resources have nonempty key, baseline, boundary, and
  readback fields; all 120 locale/operation/state fixture combinations are
  present and cardinality-consistent.
- Static CSS scanning found no font size below 12px. Semantic tokens still
  cover system, explicit light, and explicit dark themes with visible focus and
  disabled states.
- Clean, warning, expired, drift, unsupported, and secret-missing each have one
  coherent icon/tone/reason/CTA/confirmability projection. Warning remains a
  single confirmation; drift/expiry/secret-missing cannot confirm; unsupported
  has no confirm control.
- The preview still explains exact affected resources/actions, conditional
  backup and manual recovery, opaque credential-reference status, privacy,
  expiry/invalidation, zero Provider/model outbound traffic, and exact
  `planId + planDigest` confirmation without recomputing form intent.
- Static sentinel scans found no external URL/resource, absolute path,
  credential value, fetch/XHR/WebSocket/beacon, or navigation side effect.
  Displayed refs and fingerprints are synthetic and safe.
- `git diff --check` passes after this review append.

No unresolved P0, P1, or P2 finding remains in the Revision-3 prototype scope.

`USABILITY_REVIEW_ROUND_3=PASS`

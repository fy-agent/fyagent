# Concise renderer design

## Gap and owners

The renderer already has shared layout and copy contracts, but secondary views repeat those facts: Skills/MCP display source badges, source prose and source rows together, plus a read-only assignment card beside assignment switches. Auth and Health introduce already-labelled sections in prose. Loading states often narrate the same query twice.

Presentation lives in `src/pages/**`, feature-aware controls in `src/shared/features/**`, and role styling in `src/app/styles/**` / existing page CSS. Those are the change boundary; hooks, native ports, DTOs and persisted configuration remain unchanged.

## Design

- Keep each page's recognizable title, toolbar, selected object and actionable status. Remove subtitles that repeat the heading or describe obvious controls.
- Skills/MCP retain one source/configuration metadata owner and the existing assignment panel, not a duplicate assignment summary. Keep copy-only paths, redaction and provenance intact. Use a flat, compact detail section instead of three competing summary cards.
- Auth/Agent/model/editor sections retain labels and meaningful distinctions; put explanations beside the actual exceptional action rather than on every normal screen.
- Preserve direct visible warnings for deletion, credential changes, billed connectivity checks, stale/unknown results and external trust steps. Never move safety-critical content into hover-only help.
- Reuse existing Collapsible only where a genuinely optional information section needs disclosure. Avoid new shared state or universal page abstractions.
- Keep body/control/caption sizes readable, use existing spacing/radius roles and maintain natural-height rows, bounded panes and reachable footers.

## Compatibility and rollback

No dependencies, transport/command changes, route changes or persisted-state changes. Existing selection, dirty blockers, focus lifecycle, action callbacks, secret clearing and native authority remain the owners. The completion repair additionally scopes Windows-only native command adapters to Windows/test builds and removes a redundant return binding; it does not change native execution authority. Review the diff for accidental logic edits. Reverting the presentation/test/SPEC patch restores the old appearance without a data migration.

## Verification boundaries

### Completion repair boundary (user-authorized 2026-09-14)

The reported baseline failures are part of this task's completion scope. Inspect
the native diff that invalidated the source seal before updating any individual
digest. The stale Codex contract currently assumes the macOS PATH helper call is
immediately after its cfg attribute; production now places both login-shell and
process PATH calls inside a macOS-only block. Test the actual block boundary and
Windows exclusion, not an unrelated adjacency fragment. Native behavior is not
changed merely to satisfy a source-text assertion.

Renderer warnings originate in asynchronous fixture completion and potentially
the test guard lifecycle. Await meaningful terminal/readback states, not arbitrary
delays. Keep the console guard live with the project's restoreMocks policy and
add regression coverage for its lifetime. Existing native commands, product state
ownership, credentials and approved visual baselines remain unchanged.

### Performance measurement follow-up

The complete traced performance suite reproduced an additional frame-budget
failure. Two untraced repetitions of the same production-build resize test meet
the unchanged threshold. Following Playwright's documented recording costs,
separate timed measurement from trace diagnostics at the existing performance
configuration owner. Keep functional traces, real animations, CPU metrics, sample
counts, geometry/cleanup checks, one worker and zero retries. No product motion
or sampling/assertion code is changed. An executable configuration contract
prevents tracing from silently returning to the timing gate.

### Native listener fixture isolation

The last aggregate repeat exposed a pre-existing native test that binds the
product's fixed default port and assumes it is available. Use the neighboring
tests' existing ephemeral-port configuration, derive its expected profile URL
from the real `start()` result and stop the test listener. This is confined to
the existing provider unit test; no production listener or takeover code changes.

Behavioral unit tests protect workflows and important warnings; source contracts prevent known repeated narration, not all bad writing. Browser tests exercise actual geometry, keyboard/focus, themes and scrolling with fixtures. Screenshots are renderer evidence only, not native macOS/Windows or real-account acceptance.

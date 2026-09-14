# Simplify renderer copy and secondary-page layouts

## Goal

Audit all eight renderer routes and secondary views; remove repetitive UI narration and metadata, simplify hierarchy without hiding safety or actions, verify behavior and layout, then update SPEC before archiving.

## Requirements

- Review all eight product routes (Agents, Auth, Health, Models, Skills, MCP, Prompts, Memory), their secondary panels, dialogs and shared chrome.
- Lead with the object, its state and available actions. Remove repeated introductions, implementation narration, placeholder descriptions and decorative metadata cards.
- Use concise Chinese labels and task-specific messages, not slogans, generic reassurance or repeated instructions.
- Preserve decision-relevant warnings, source/provenance, configuration impact, errors, unknown/stale states, accessibility, keyboard access and existing workflows.
- Keep the existing design system, navigation, theme and motion owners. Do not introduce dependencies, native behavior changes or data migrations. The approved completion scope below includes mechanical native compile repairs.
- Update the owning SPEC before archiving this task, with reproducible verification and explicit evidence limits.

## Acceptance Criteria

- [x] Every product route and secondary-view family has an audit disposition recorded.
- [x] Skills/MCP details no longer duplicate the editable assignment state or repeat source/transport facts across badges and cards.
- [x] Account, Agent, model, prompt and memory secondary views use compact headings and only relevant help; safety and recovery information remains available at the point of action.
- [x] Loading/empty states do not repeat the same message in a subtitle.
- [x] Shared layout stays readable and usable in narrow/wide containers, both themes and Chromium/WebKit; no hidden controls or smaller body text used to fake simplicity.
- [x] Renderer-specific checks pass with the repaired warning guard; the previous full functional browser run passed 586 tests. Final repeat results are recorded in `verification.md`.
- [x] Affected tests, frontend gate, browser checks, build and task/SPEC contracts pass; remaining native/manual limitations are stated.
- [x] Owning frontend SPEC updates are prepared and reviewed before task archive.
- [x] Archive the task after the authorized local work commits; validate the relocated context and pass the canonical full check without exclusions.

## Non-goals

No new features, API or storage changes, dependency updates, route redesign, native installs/logins, release or remote publication.

## Authorized completion scope

On 2026-09-14 the user requested that all reported remaining problems be resolved,
not archived under the previously proposed aggregate-gate exception. The user also
authorized the proposed local commit and archive; remote publication remains out
of scope. Continue this existing task rather than create a competing active task.

- Resolve the three baseline contract failures after reviewing their actual source
  changes; preserve Windows Shell-user authority and strict platform inventories.
- Resolve React `act(...)` warnings at their test lifecycle owner and ensure the
  warning guard remains active across tests. No warning suppression or assertion
  weakening is acceptable.
- Run the full local prearchive check, release contracts, complete functional
  browser matrix and serial production performance suite, then update SPEC,
  commit locally, archive and validate without an active-task exclusion.
- Keep real-account, Windows/macOS installer/signing and live service acceptance
  distinct from local source, native-host and fixture evidence.

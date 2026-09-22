> 归档说明：本文的时间与结论保留；具体工作站用户目录已替换为语义占位符。历史路径映射、原稿及提交稿哈希见[归档路径映射](research/archive-path-map.json)。可解析的相对链接已调整。

# Browser final review — 2026-09-22

Owner: native_continuation. Product write scope was limited to
`tests/browser/state-motion.spec.ts`; no motion component, renderer fixture or
product stylesheet was changed by this work package. Browser runs were serial,
with one repository-managed server at a time.

## Deterministic failure and correction

The original failure at 1440×900 was a readiness error in the test. The test
waited for a panel with `height: auto` but did not wait for the asynchronous
model ID query. The empty-state panel also has `height: auto`.

A controlled local test held `get_workbuddy_model_ids` unresolved, opened the
same disclosure, and measured the original failing value exactly:

- Before releasing the query: 23.59375 px, scrollHeight 24, text
  `还没有找到已配置的模型 ID`.
- After releasing the same query: 124.984375 px, scrollHeight 125, text includes
  `搜索已有模型 existing 1 existing-model`.

The relevant Models, Collapsible and fixture files are unchanged from baseline
`c0b2ec21` (read-only Git diff checked). No baseline checkout was reset or
modified. This is source comparison plus a deterministic delayed-response
counterexample, not a claim that a separate full main suite was run.

The production test now waits for the fixture's actual `existing-model` chip
before opening/freezing the clock. Its closing start must match actual
`scrollHeight` within one pixel. The requirement for more than two intermediate
closing frames remains, and the test now additionally requires the final sample
to be below one pixel. The fixed 25 px threshold was not lowered; it was replaced
with the intended complete-content geometry condition.

Formal regression:

```text
rtk proxy mise run format:files -- tests/browser/state-motion.spec.ts
rtk proxy mise run test:browser -- tests/browser/state-motion.spec.ts
```

The production boot pre-check passed and all **25 state-motion tests passed**
across four Chromium viewport sizes and WebKit. This is a focused regression;
the earlier full-suite result of 666/667 is not relabeled as a full-suite pass.
Logs and the controlled counterexample are copied to
`evidence/browser-final-review/`.

## Screenshot procedure and boundaries

Scratch `work/browser-final-review/screenshots.spec.ts` copies the existing
`installSessionMigrationFixture` function and adds local-only package/read/write
responses through its fixture IPC delegate. Production fixtures are unchanged.
All histories, paths, IDs, provider versions and results in these screenshots
are synthetic. No real history was loaded, native store written, or model
called. This is renderer evidence, not native restore/UAT evidence.

Two Chromium sizes, 900×600 and 1440×900, cover:

1. Session list with two distinct same-body sources and an unfinished turn.
2. Import package review and target-software selection.
3. Workspace/conflict options with the action footer reachable.
4. A returned `nativeWritten` result that is still awaiting native readback.

User-facing images are under `outputs/session-recovery-review/` outside the
repository. The actual images were opened and visually inspected.

## Findings during inspection

- The first capture run caught a real presentation issue: `nativeWritten` was
  grouped with `nativeReadbackVerified` as `cleanSuccess`, producing a green
  `恢复执行完成` banner despite the card saying `待读回验证`. This was reported to
  the coordinator; pre-fix screenshots were preserved as evidence. A local
  screenshot assertion deliberately failed rather than treating this as
  verified success.
- At 900×600 the list/detail layout becomes vertical. The provider filters and
  header leave only a narrow list strip above the selected conversation. The
  second source remains reachable by the list's scroll interaction, but the
  view is dense; this is a usability observation, not evidence that the second
  record disappeared.
- The initial 1440×900 detail view showed a Codex implementation explanation
  containing `Store` and historical implementation claims, plus unstyled
  native-looking action buttons for readback/manual continuation. These were
  reported for coordinator-owned cleanup; this agent did not change product UI.
- The import dialog has a usable internal scroll region and an action footer
  within the 900×600 viewport. The selection, target directory and conflict
  choices remain operable. The extra options screenshot documents content
  below the initial fold instead of claiming the whole form fits at once.

## Final capture after coordinator fixes

The coordinator corrected the pending-write classification, removed the
implementation/history assertions from the Codex note, and replaced the two
plain action buttons with the shared Button component. After that UI freeze,
the two-size screenshot run passed **2/2**, with no page errors. Its explicit
assertions require a non-success banner for `nativeWritten`, the text
`已写入，待读回验证`, and the `待读回验证` per-attempt stage.

Both viewport screenshots were opened after this final run. The 1440×900
detail view now shows a yellow pending-readback notice and consistent buttons;
both result dialogs state that complete history is still unverified. The
900×600 dense list layout remains as the documented usability observation.
The body dimensions equal the viewport dimensions at both sizes, without
horizontal document overflow; the result dialog and action controls stay
inside the viewport. Native reply generation remains untested by these images.

Final command:

```text
rtk proxy mise exec -- pnpm exec playwright test --config /Users/<username>/Documents/Codex/2026-09-22/new-chat/work/browser-final-review/screenshots.config.ts
```

Final output contains eight PNGs (four states × two viewports) and a short
fixture-only README in `outputs/session-recovery-review/`. The final
`screenshots.log`, deterministic counterexample, and focused formal regression
log are under `evidence/browser-final-review/`. All servers created by these
completed runs have exited.

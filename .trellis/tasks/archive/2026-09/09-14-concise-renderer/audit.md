# Renderer audit

## Scope and decisions

All eight production route families, their secondary panels and shared dialogs
were reviewed. This is a presentation change, not an authentication, installation,
configuration or storage redesign.

| Surface | Removed or consolidated | Deliberately retained |
| --- | --- | --- |
| Agents | Duplicate loading/error narration; missing-description filler; verbose CLI login confirmation | Product sections, actual assignments, retry actions, install-source/authorization distinctions, credential ownership and cancellation limits |
| Auth | Generic page/selection introductions; repeated connection count and identical target name; dedicated danger-introduction card; nested section boxes | Account identity versus model/request source, actual software cards, distinct target labels, removal preview, official login/credential/file impact |
| Health | Page/sort/loading instructions; duplicate unchecked count; routine success paragraph repeating the status and remote-service limitation | All 12 checks and 13 timestamps, non-ready reasons, stale/read failures, explicit remote-service/usage limitation, check/stop/retry actions |
| Models | Duplicate Qoder/TRAE unsupported notices; repeated overwrite confirmation; long selection/usage narration | Unsupported/read-only behavior, actual selected IDs, unsaved state, irreversible deletion, billed connectivity test, experimental subscription and background-process limits |
| Skills | Three competing source/assignment/date cards become one flat installation section; absent descriptions and repeated loading text omitted | Source badge once, repo/branch/path/dates/docs, copy-only directory, real seven-target switches, uninstall impact; no guarantee of an optional backup |
| MCP | Same one-section detail; no empty directory row, duplicate transport/source or assignment summary | Commands and redacted arguments/URLs, env/header counts, ID/provenance, editor secrets confined to their existing owner, real assignments and WorkBuddy trust instructions |
| Prompts | Loading/selection subtitles, absent description filler and verbose discard text | User content, actual enable controls, native-only limits, current-file inspector, dirty/discard/delete checks and navigation authority |
| Memory | Normal `已读取` badges, redundant loading/selection narration and wordy discard text; header uses section token and toolbar avoids unnecessary vertical stacking | Missing/dirty/error/stale states, four fixed resources, full user text, per-resource limits, file paths, daily search and real actions |

## Shared presentation

The existing role scale is retained: 14px body, 13px control, 12px metadata and
16px section headings. Skills/MCP descriptions are ordinary body text, not
another heading. Metadata uses its existing definition/copy layout and narrow
container behavior, with a leading divider instead of nested summary cards.
Real assignment controls remain visible; removing a duplicate read-only summary
does not remove supported or hidden native assignment flags.

Do not interpret this task as a prohibition on all help, all subtitles or all
repeated strings. A selected object must remain identifiable in master and detail;
a warning may need to appear at both its summary and action point. Distinct
connection targets, failure causes and native-safety facts are not filler.

## Review findings corrected

- Merging metadata exposed an imprecise test: an absolute MCP command legitimately
  contains the installation directory. Copy-only path assertions now scope the
  actual path control, while command rendering and secret redaction remain tested.
- A normal Auth fixture includes an outstanding official login. Screenshot
  readiness waits for actual page content and excludes loading empty states,
  not all spinners; product session behavior was not changed to satisfy a test.
- Visual review found identical `Codex` heading/subtitle and an unfiltered
  `40 / 40` account count. Those repetitions were removed; a distinct target
  label and meaningful filtered count are still displayed.
- Visual review found normal Health status followed by another success paragraph
  repeating the remote-test limitation. The normal paragraph is omitted while
  the scope warning, non-ready reasons and all check rows remain.
- Memory actions were shrinking into a tall stack despite sufficient total pane
  width. The copy-only path control inherited the metadata-row `width:100%`.
  It is now auto-width in the toolbar, and the existing flex header wraps the
  intact toolbar as a group; controls still wrap inside a truly narrow pane.

## Evidence limits

Browser fixtures exercise renderer DOM, layout, actions and accessibility; they
are not native macOS/Windows acceptance or live-account tests. No real login,
installation, credential change, model request or native configuration write was
performed. No release, approved visual baseline or remote branch was published.

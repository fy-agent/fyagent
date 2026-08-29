# #141 current-main frontend mapping

Issue: https://github.com/fy-agent/fyagent/issues/141

This file is a planning classification, not final UAT evidence. Every historical item still requires latest-main browser/native retest.

| Finding | Current planning status | Stage 5 action |
| --- | --- | --- |
| A3 Search/Settings/Account inert | Code-confirmed still applies | Wire, remove or explicitly disable; no focusable noop |
| User-reported left selected state dims | Current architecture risk; native repro pending | CSS-first host state + Lens failure/delay tests |
| A4 Models sticky overlap | Needs current-main retest | Recheck minimum window/long form during route/layout work |
| A5 MCP last target/scroll affordance | Needs current-main retest | Recheck pointer/keyboard visibility and scroll cue |
| A6 Prompts app rail discoverability | Needs current-main retest | Recheck rail sticky/scroll behavior after route unmount change |
| A7 duplicate search clear | Needs current-main retest | Keep one FeatureSearch owner/native clear policy |
| A8 corrected validation style | Needs current-main retest | Verify touched/dirty/corrected lifecycle in form tests |
| B4 DPI/minimize/full keyboard evidence | Evidence gap | Requires exclusive Windows native session; browser tests do not close it |
| B7 untouched model validation | Needs current-main retest after later model work | Test untouched/touched/corrected/submit/save lifecycle |
| B8 Skills Discover header actions | Needs current-main retest | Verify Installed/Discover composition and keyboard access |
| React `act(...)` warnings | Current test-output confirmed | Fix timing/cleanup; no suppression |
| Prompt live-file and Daily Memory blockers | Outside Stage 5 | Must remain visible under Stage 0/#141 remediation |

Completion requires replacing this planning status with exact SHA, environment, test/UAT evidence and `fixed | still applies | obsolete` for every in-scope row.

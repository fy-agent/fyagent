# Windows UAT issue register

This register contains Windows-only findings from the installed FyAgent 0.4.0
run. A blocked test-evidence item is distinguished from a confirmed product
defect. Suggested owners are routing recommendations, not assignments.

## Summary

| Id | Severity | Type | Status | Finding | Release effect | Suggested owner |
| --- | --- | --- | --- | --- | --- | --- |
| WIN-UAT-001 | P1 | Product defect | Confirmed | Daily Memory fails the whole page and Retry fails again when the current directory contains valid and non-date Markdown files. | NO-GO | Memory / OpenClaw frontend + backend |
| WIN-UAT-002 | P1 | Release provenance | Confirmed | Installed FyAgent executable, user helper, and uninstaller are all unsigned. | NO-GO | Windows packaging / release |
| WIN-UAT-003 | P1 | Testability / safety | Blocked | Formal installed build has no supported disposable-profile redirect, blocking both mandatory isolated P1 retests. | NO-GO until evidence exists | Windows runtime / UAT infrastructure |
| WIN-UAT-004 | P1 | Acceptance evidence gap | Blocked | 125/150% DPI, stable minimize, and full keyboard focus order could not be safely captured on the shared desktop. | NO-GO for full Windows sign-off | Windows UAT environment owner |
| WIN-UAT-005 | P2 | UI contract | Confirmed | Six non-Codex Agent details render no visible 产品介绍 section and leave a large empty body. | Fix before next candidate | Agent page |
| WIN-UAT-006 | P2 | Content contract | Confirmed | Installed Codex installer explanatory copy names FyAgent contrary to the current contract. | Fix before next candidate | Agent / Codex installer UI |
| WIN-UAT-007 | P2 | Form UX | Confirmed | Untouched WorkBuddy, Grok Build, and OpenCode model forms show validation errors before interaction. | Fix before next candidate | Models UI |
| WIN-UAT-008 | P2 | UI contract | Confirmed | Skills Discover removes 检查更新 and 更多 although the header contract keeps them mounted. | Fix before next candidate | Skills UI |
| WIN-UAT-009 | P2 | Local-tool provenance | Needs clarification | Grok Bot registry publisher is SpaceXAI while the valid executable signer is Anysphere, Inc. | Conditional: clarify provenance | Tool owner / desktop security |
| WIN-UAT-010 | P2 | Local-tool inventory drift | Confirmed | Multiple tools expose conflicting registry, executable, CLI, or parallel-install versions. | Conditional: define detection precedence | Agent detection / integrations |

## WIN-UAT-001 — Daily Memory whole-page failure

**Observed Windows reproduction**

1. Open Memory, then Daily Memory in the installed app.
2. Wait for the initial loading state.
3. Observe “无法加载每日记忆 / 请稍后重试”.
4. Click Retry once.
5. Observe loading, followed by the same terminal failure.

**Actual:** No valid daily entry is usable after the first load or Retry.

**Expected:** Valid YYYY-MM-DD.md entries remain listable. A non-date Markdown
file should be ignored, partitioned into a non-daily area, or reported without
invalidating the complete list.

**Evidence:** EV-040–EV-043, SYS-008, SRC-002.

The read-only current-profile shape contains 2 valid-date and 6 non-date
Markdown files. Source audit explains the failure path: the backend returns all
.md entries, while the frontend maps the whole response through strict
real-date filename validation. This is a Windows runtime failure on the current
profile, but it is not mislabeled as the required synthetic isolated AC7 retest.

**Retest:** In an authorized disposable Windows environment, seed exactly one
valid date file and one non-date Markdown file. Verify the valid item loads,
Retry recovers, the invalid item is handled explicitly, and authoritative
filesystem state is unchanged.

## WIN-UAT-002 — Installed FyAgent artifacts are unsigned

**Actual:** Fresh Authenticode readback reports NotSigned for all three files:

| Artifact | Version | Bytes | SHA-256 |
| --- | --- | ---: | --- |
| fyagent.exe | 0.4.0 | 32,268,288 | A2F61EB08DEB9CC7AD997E5750647FD202C8949DBECD4338DA48C94BCECC0F79 |
| fyagent-user-helper.exe | no file version | 202,752 | 70F1B2EAE181F61DB1A2F9D77906F1CE0F9DF024B6FDE4365DD39F564FFADC50 |
| uninstall.exe | 0.4.0 | 96,546 | 0579B9DB3E0FC4995792C6965FFEFAE3C21A6DE46B5DB5AA989DA11F2116BA8C |

**Expected:** A machine-wide desktop candidate and its privileged/helper and
uninstall boundaries have an attributable, valid Authenticode chain.

**Evidence:** SYS-002.

**Retest:** Sign the exact release artifacts, install the candidate on a clean
Windows test machine, and verify chain, timestamp, publisher, hash, SmartScreen
boundary, install, launch, update, and uninstall. No Defender exclusion should
be required.

## WIN-UAT-003 — No supported safe redirect for installed-profile writes

**Actual:** A private rollback copy and an isolated working copy were created;
both have the same 2,994-file aggregate fingerprint and zero reparse points.
That proves the two private copies match each other. It does not make the
installed app use them.

The formal Windows home resolver ignores FYAGENT_TEST_HOME unless the build is a
test or test-hooks build. The installed binary contains no FYAGENT_TEST_HOME or
test-hooks marker. Windows Sandbox is disabled/absent, Hyper-V has no existing
VM, and no authorized disposable account/session was available.

**Blocked hypotheses, kept separate**

- Prompt hypothesis: **NOT TESTED**. Current-source code audit shows that saving
  a disabled prompt when none is enabled clears an existing live prompt file,
  but no installed-runtime write was made.
- Daily Memory mixed-name hypothesis: **NOT TESTED as an isolated retest**.
  WIN-UAT-001 independently confirms a read-only current-profile failure, which
  is not an AC7 substitute.

**Evidence:** SYS-004, SYS-005, SRC-001, SRC-003.

**Minimum unblock:** Supply one of: an already-enabled disposable Windows
Sandbox/VM; an installed 0.4.0 test-hooks package with vendor-supported profile
redirect; or a vendor-sanctioned HIL profile-redirection mechanism. Then record
pre-state, C/R/P/A, post-state, and rollback fingerprints for each hypothesis.

## WIN-UAT-004 — Required scale/minimize/focus evidence is missing

**Actual:** Windows Settings launched through its explicit executable but did
not expose a targetable Computer Use window. One minimize result was transient
and contradicted by immediate native readback. Computer Use then detected real
user input on three attempts; all further GUI input was stopped.

**Not tested:** 125%, 150%, stable minimized state, full Tab/focus order.

**Covered:** 100% normal, maximize, restore, scrolling, native title bar,
representative long Chinese text, and absence of horizontal clipping in those
states.

**Evidence:** SYS-003, SYS-006, EV-049, EV-050.

**Minimum unblock:** An exclusive Windows desktop session where system display
scale may be changed through normal Windows UI and the app can be restarted at
each scale, with no concurrent user input. Do not substitute browser zoom,
WebView zoom, registry writes, or forced PowerShell scaling.

## WIN-UAT-005 — Agent details omit 产品介绍

**Actual:** TRAE, QoderWork, WorkBuddy, Grok Build, Claude Code, and OpenCode
show their identity/header and a large blank detail region, with no visible
产品介绍. Codex legitimately uses its installer body instead.

**Expected:** The current Agent contract requires page-local 产品介绍 on each
non-Codex detail.

**Evidence:** EV-001, EV-003–EV-005, EV-008, EV-009, SPEC-001.

**Retest:** Inspect all six non-Codex entries at 100/125/150%, normal and
maximized, verifying introduction content, wrapping, scrolling, and keyboard
focus.

## WIN-UAT-006 — Codex installer copy names the host

**Actual runtime copy:** “在 FyAgent 中安装、更新或启动桌面应用。”

**Expected:** The current contract says third-party directory/installer copy
describes Codex Desktop itself and must not name FyAgent.

**Evidence:** EV-006, EV-007, SPEC-001.

**Retest:** Verify installed candidate copy at loading, ready-to-launch,
ready-to-update, unavailable, and failed states.

## WIN-UAT-007 — Validation shown before interaction

**Actual:** Untouched drafts immediately render:

- WorkBuddy: at least “请至少添加一个模型 ID”.
- Grok Build: base URL, API key, and model-ID errors together.
- OpenCode: blank service URL error in the existing-provider view.

No secret was entered and no Save was clicked, so backend rejection or
persistence is not claimed.

**Evidence:** EV-010, EV-013, EV-016, EV-017.

**Expected:** Existing configuration remains readable; a new/empty draft should
not show destructive-looking validation until the relevant field is touched or
submission is attempted.

**Retest:** Cover untouched, touched-invalid, corrected, cancel, submit-failed,
and successful authoritative persistence states in a disposable profile.

## WIN-UAT-008 — Skills Discover drops header actions

**Actual:** 检查更新 and 更多 are present on Installed and disappear on Discover.

**Expected:** The current contract keeps the Skills header mounted so both
actions remain on Discover.

**Evidence:** EV-018, EV-019, SPEC-002.

**Retest:** Switch Installed↔Discover repeatedly at all required DPI values;
verify both actions remain mounted and keyboard reachable without causing
unexpected requests.

## WIN-UAT-009 — Grok Bot publisher/signer mismatch

**Actual:** The per-user uninstall record identifies Grok Bot 0.24.0 publisher
as SpaceXAI. The executable has a valid Authenticode signature from
Anysphere, Inc. Nine Grok Bot processes were running during the fresh inventory.

This register does not label the binary malicious. The mismatch requires an
authoritative provenance explanation before treating the registry publisher as
the software signer.

**Evidence:** SYS-007.

**Retest/triage:** Confirm the expected distribution channel and signer with the
tool owner, compare the installer/executable hashes to that channel, and correct
publisher metadata or packaging if unintended.

## WIN-UAT-010 — Tool version and parallel-install drift

**Observed**

- Qoder CN: uninstall version 1.106.3, main executable 1.25.1.
- CodeBuddy: uninstall version 1.106.1, main executable 4.10.0.
- OpenCode: legacy uninstall 1.2.26 with unsigned executable version 0.0.0;
  CLI 1.18.14; separate signed desktop 1.18.18.

**Risk:** Any future “installed”, “current version”, launch, or update detection
can choose a stale registration or the wrong binary unless precedence and
product identity are explicit.

**Evidence:** SYS-007.

**Retest:** Define authoritative detection precedence, then exercise single,
parallel, stale-registry, PATH-shadowed, and uninstalled-current cases. FyAgent
0.4.0 did not expose detection state for these entries, so no existing detection
pass is claimed.

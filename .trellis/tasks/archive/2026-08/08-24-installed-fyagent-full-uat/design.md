# UAT Design

## Boundaries

This is an evidence-producing review task, not a product implementation task. The installed application is the runtime system under test. The current `origin/main` checkout supplies route, state, and native-contract context only.

Raw runtime evidence lives outside Git under a task-specific local directory. The repository receives the task contract, a sanitized evidence index, and the final report. Any raw evidence containing user-specific values is marked restricted and represented in Git by an id, timestamp, SHA-256, capture method, and redacted observation only.

## Evidence Flow

1. Record immutable runtime and repository baselines.
2. Enumerate code-declared routes/commands and runtime-visible surfaces independently.
3. Back up application data before persistent-risk interactions.
4. Capture a baseline screenshot and accessibility state for each page/state.
5. Execute safe interactions and immediately re-read the resulting UI and, where safe, authoritative local state.
6. Write one sanitized evidence record per capture/interaction.
7. Derive coverage, visual scores, functional results, findings, and verdict from the evidence index.

## Safety Contracts

- No real secret, token, private body, or sensitive path is copied into Git or transmitted.
- No persistent write occurs unless the target, pre-state backup, isolation label, and rollback/readback path are known.
- A successful click is never upgraded to persistence or authoritative readback without direct evidence.
- Missing runtime access is reported as an untested boundary or blocker, not inferred as pass.
- Screenshots are captured after the tested surface is stable; later product/source changes are out of scope and would invalidate affected evidence.

## Compatibility and Limitations

- Platform claim: `macOS` only.
- Host: Apple Silicon macOS, installed universal app, version 0.4.2.
- Windows-native layout and provider behavior remain outside this run.
- No `pixel_diff` evidence is planned because no canonical design image is supplied.
- Network/provider behavior is evaluated only where the installed app exposes a safe, credential-free path.

## Windows Reuse Boundary

The later AIMaster/Windows reviewer may reuse the route checklist, evidence grades, functional-layer vocabulary, issue schema, and report layout. It must independently inventory the installed Windows build and all detected Agent tools (including GrokBot, Codex, and any Windows-only tools), verify Windows package provenance/signing and launch state, and repeat every platform-sensitive page, write, readback, resize/DPI, filesystem, permission, native-unavailable, and failure-path case. A macOS pass or failure is input to Windows testing, never Windows acceptance evidence.

## Rollback

- The source worktree can be discarded independently without touching existing branches.
- User data backups are created before any allowed write; rollback is verified only when a reversible write is actually executed.
- Product code is not modified, so there is no product rollback path in this task.

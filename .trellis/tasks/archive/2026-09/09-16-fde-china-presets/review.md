# Review

## Product and content pass

Thirty original Chinese templates cover six groups with five distinct tasks each. Reviewed every scenario's inputs, implementation slice, deliverables, acceptance cases and boundary example, rather than judging quality by character count or changing a generic role name. The common contract supplies evidence, tool approval and privacy boundaries while allowing phase-appropriate execution; it does not demand a fixed report, invented ROI or hidden reasoning disclosure.

The content covers discovery through production handoff, domestic office/ERP integration and China-oriented industry operations. Regulated scenarios explicitly preserve professional review and current-source verification. Environmental-report content never fills missing measurements from a reference template. This is a content review, not a benchmark demonstrating a measured LLM success rate.

Eight new MCP recipes complement twelve existing FDE-relevant entries, rather than duplicating domestic collaboration tools. Their upstream ownership, launch instructions, credential fields, runtime requirements and risks were reviewed separately. Official documentation supports the recipes; no customer-side login or connection was tested. The research record preserves rejected and deferred options.

## Engineering and security pass

- Preview and catalogue selection are read-only. Applying creates an independent unsaved draft; Save creates a disabled library record; Enable remains explicit. All seven application libraries retain their existing native authority and readback behavior.
- Dirty edits survive browsing. Replacing them requires the existing confirmation; cancellation preserves user text. No second write controller, installer, state store, dependency, native command or migration was introduced.
- Replaced timestamp uniqueness with a fresh 128-bit opaque ID. An intermediate implementation used crypto.randomUUID; the final implementation uses crypto.getRandomValues to avoid a new secure-context/WebView API dependency. Existing persisted IDs are untouched. A regression test disables randomUUID and proves creation still succeeds.
- New secret fields enter env, never argv or ordinary search. DMS, DBHub and StarRocks remain labelled write-capable; RDS remains cloud-privileged even with its write-tool flag off. DataWorks and CloudOps use documented restricted tool selectors; ACK omits --allow-write. Labels and prompts are not authorization controls.
- No third-party MCP process, cloud deployment, billing operation or customer credential was used in repository verification. Version selectors are upstream-owned and must be revalidated when recipes change.

## Interaction and regression pass

Reused the shared tabs, search, list, split panes, buttons and confirmation dialog. The preview text stays editable only after explicit copying. Empty filter results expose a clear action. Browser-only mode can read static templates but never claims a writable native library.

Found and fixed a confirmation focus issue: the source button unmounts after applying a preset. Its captured return anchor is now the persistent preset tab, allowing the shared dialog to return to the currently active sibling tab after confirmation. No new shared focus workaround was added.

The initial focused test run found stale timestamp-ID expectations, an arbitrary template-length assertion, and a native-query timeout shorter than the existing retry window. Corrected these assertions without relaxing functional, secret, identity or layout boundaries. The first browser run was blocked before tests by missing Playwright browser binaries; installed the locked test runner's Chromium/WebKit runtimes, then reran the gate.

Added the FDE browser file to WebKit's explicit test selection as well as the ordinary Chromium suite. Viewport tests inspect the actual preview bounds and readable height, not merely the absence of console errors. Screenshots are test artifacts, not approved native visual baselines.

The full browser matrix initially passed 623 cases and failed WebKit prompt scroll reachability. The same failure reproduced in three serial runs while both new preset viewport cases passed. The wheel helper used a fixed 80px vertical offset even for a shorter eligible owner. Changed only the test helper to use the actual height and assert elementFromPoint hits the owner/descendant before dispatching the physical wheel. The three serial repetitions then passed all nine cases without changing product scrolling or reachability assertions. The durable physical-input rule was added to quality-guidelines.md.

A late UUID-availability regression initially selected one of two identical empty-state/header New buttons ambiguously; changed the test to use the unique preset action being validated. A concurrent low-priority aggregate run also hit unrelated contract timeouts; those failures are not counted as passes, and the final aggregate is run serially with the normal priority and unchanged timeouts.

## Evidence and residual boundaries

Exact final commands and results are recorded in verification.md. All review passes were performed inline by this agent; no independent subagent reviewer is claimed because this DevSpace advertises no agent providers. Mock/native-port tests and browser automation are not native desktop HIL, Windows installer acceptance, authenticated MCP smoke or LLM output evaluation.

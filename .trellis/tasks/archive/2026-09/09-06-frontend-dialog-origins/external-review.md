# Independent integration review

Reviewed current source without editing the ongoing Dialog implementation.

1. `runDialogPresentation` currently lets a matching captured snapshot override
   `dialogOriginGeometry(...).sourced` unconditionally. A transient menu click
   followed by an asynchronous operation and a window resize can leave its
   captured rectangle outside the current viewport. Please add a deterministic
   regression and reject unusable/out-of-viewport captured geometry before the
   `sourced:true` override. The old live-source checks alone cannot protect this
   snapshot path. Keep neutral/current explicit return-anchor fallback; never
   infer another last-click source. This is a geometry robustness issue, not
   evidence of a credential or permission bypass.
2. Supplemental all-seven-route physical scrolling is now committed in3cc34ad7:
   50 cases passed across four Chromium viewports and WebKit. Main entry/test
   changes from that commit must survive root-configuration integration.
3. Root work remains isolated onrefactor/round7-root-governance. Its code/test
   contract changes are being verified there. Do not archive the parent until
   those commits are merged and the merged checkout passes full gates.

## Completion coordination

The explicit WorkBuddy follow-up source was also missing after MCP catalog
installation and editor save: Discovery had its own source ref, whereas the
outer trust notice read the unrelated page ref. These source handoffs now pass
the initiating operation's ref, with the final install/save event captured
before its dialog retires. The added `mcp-followup-origins.spec.ts` covers both
paths with stateful synthetic writes and must pass alongside dialog-origins.
Both browser configurations include it. No real MCP/credential write is used.
The earlier full-browser process started before this added suite; do not treat
that process alone as the final gate for these additions. Root-governance
work and final merge/cleanup remain owned by the isolated-worktree continuation.

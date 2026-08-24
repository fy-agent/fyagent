# Agent install readiness consolidation

## Goal

Integrate only the truthful read-only readiness outcome of #115 with the canonical seven-entry Agent Catalog, without creating an installer or second catalog.

## Requirements

- Accept exactly `qoderwork / trae-work / workbuddy / grokbuild / codex / claude-code / opencode`.
- Preserve distinct source/license, integrity, preflight and plan layers; `fail` and `unknown` never become green.
- Add exactly one read-only native command contract: `get_agent_install_readiness`.
- Return only ID, states, reason codes, timestamps and sanitized summaries; exclude URL/path/hash/script/secret/package/signer fields.
- All generic automation is unavailable. Codex uses `managed_by_codex_desktop`; the existing Codex Desktop installer remains unchanged.
- Add a compact read-only “安装方式” region to the existing `/agents` detail; no Settings tab or action button.
- Do not modify shared command/lib/ACL registration or FeaturePorts composition; integration owns those files.

## Acceptance Criteria

- [x] Exact seven-ID alignment and unknown-ID rejection pass.
- [x] Four-layer rollups preserve fail/unknown semantics and TTL/drift contracts without creating a snapshot.
- [x] Wire DTO exact-key and sensitive-field negative tests pass.
- [x] Only one read-only command exists; no start/get_job/cancel/probe/doctor/helper surface exists.
- [x] Agent detail has no install/recheck/cancel/health button and Codex installer does not regress.

## Notes

- Source PR: #115. Current Agent Catalog v4 is authoritative.

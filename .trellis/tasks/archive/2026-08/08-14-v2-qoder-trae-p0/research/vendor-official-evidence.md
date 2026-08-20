# Vendor Official Evidence

Reviewed: 2026-08-14

## QoderWork

- Skills: https://docs.qoder.com/qoderwork/skills
  - Each Skill is a directory containing `SKILL.md` under `~/.qoderwork/skills/`.
  - The official documentation permits filesystem add/remove/edit.
- Hooks: https://docs.qoder.com/qoderwork/hooks
  - User-level hooks live in `~/.qoderwork/settings.json` under `hooks`.
  - Hooks are executable command logic; validation must never run them.
  - Hot reload is not supported; QoderWork must restart after edits.
- Connector: https://docs.qoder.com/qoderwork/connectors
  - Custom MCP servers are managed through the vendor UI with manual/JSON import.
  - No stable third-party private-storage write contract is documented.
- Installation:
  - https://docs.qoder.com/qoderwork/install-windows
  - https://docs.qoder.com/qoderwork/install-macos
  - macOS documents `/Applications/QoderWork.app`; Windows documents user/system install scopes but not a complete stable executable/signing identity contract.

## TRAE Work CN

- Skills: https://docs.trae.cn/work_skills
  - Project Skills use `.trae/skills/`.
- Current TRAE Work global Skills use `<home>/.trae-cn/skills` on supported desktop platforms.
- Models: https://docs.trae.cn/work_models
  - Desktop supports custom model API format, URL, model ID and authentication input.
  - The vendor performs its own service check at final submission.
- MCP: https://docs.trae.cn/work_remote-mcp-server
  - stdio uses command/args/env; HTTP uses url/headers.
  - Values may contain credentials.
- These pages do not publish a stable third-party local model/MCP storage write contract. FyAgent therefore validates and guides but does not write vendor private storage.

## Security and Platform

- Tauri capabilities: https://v2.tauri.app/security/capabilities/
- MCP security best practices: https://modelcontextprotocol.io/docs/draft/tutorials/security/security_best_practices
- User-controlled URLs require strict address, redirect, credential and consent controls. New MCP servers/commands must not be executed or enabled from untrusted content.

## Evidence Boundary

- Official documentation supports static paths and feature semantics.
- Automated code/tests support FyAgent-owned validators and safe file operations.
- No real desktop HIL is in scope. App recognition, launch identity, vendor Skill recognition and Hook effectiveness remain `unverified`.

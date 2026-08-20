# Catalog identity evidence (2026-08-18)

## TRAE Work CN

- Catalog currently serializes `displayName: "TRAE Work"` and product URL `https://work.trae.cn/` in `src-tauri/src/commands/agent_catalog.rs`.
- Required product URL is `https://www.trae.cn/sem-work`.
- Required visible name replaces every remaining "TRAE Work" product string with **TRAE Work CN** (same pattern as QoderWork CN). Stable ids stay `trae-work` / `trae-work-cn`.
- Local desktop app on this machine is `/Applications/TRAE SOLO CN.app`. Home skills dir is `~/.trae-cn/skills`.

## QoderWork CN icon

- Current V2 asset `src/v2/shared/assets/agents/qoderwork.svg` is the dark cube / green crystal mark (Qoder IDE family), not QoderWork CN.
- macOS app `/Applications/QoderWork CN.app/Contents/Resources/icon.icns` is a bright green squircle with a two-eye cartoon face and a spiral brow. That is the mark to ship.
- Extract with `sips -s format png` to a V2-owned PNG (256px). Do not keep the cube SVG as the catalog icon.
- Home dir on this machine is `~/.qoderworkcn` (skills live there). Backend Skill copy currently uses `~/.qoderwork/skills`; do not silently retarget in this task unless a focused test proves the live CN path. Call out any mismatch in the Agent detail as unknown rather than inventing a second skills root.

## QoderWork CN models

- Catalog already declares `models.validate` / `models.write` as `unsupported`.
- Models page still presents QoderWork as if FyAgent/QoderWork can finish model setup (`模型、Hooks 和 MCP`, “在 QoderWork 中完成模型设置”). That copy is wrong.
- `~/.qoderworkcn/.status.json` has `allow_byok: 1`, but the user instruction is authoritative: third-party model configuration is **not supported**. Models page must say 不支持, not “去应用里完成”.

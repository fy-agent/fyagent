# Vendor MCP live paths

Reviewed: 2026-08-19 on this machine's installed apps.

## QoderWork CN 0.45.2

`/Applications/QoderWork CN.app/Contents/Resources/app.asar` → `out/main/chunks/constants-DrlprGB5.js`:

- home = `join(homedir(), ".qoderworkcn")`
- `exports.m` joins home + extra segments
- `CUSTOM_MCP_CONFIG_PATH = constants.m("mcp.json")` → `~/.qoderworkcn/mcp.json`
- `writeCustomMcpConfig` writes `JSON.stringify({ mcpServers })`
- `getBuiltinMcpConfigPath()` = `app.getPath("userData")/mcp.json` (Application Support). Do not write.

Local home exists: `~/.qoderworkcn/skills`. No custom `mcp.json` yet.

Plugin examples ship `{ mcpServers: { name: { type, url } } }` as `.mcp.json`.

## TRAE SOLO CN

Forum (docs.trae.cn work_remote-mcp-server + community):

- macOS `~/Library/Application Support/TRAE SOLO CN/User/mcp.json`
- Windows `%AppData%\Roaming\TRAE SOLO CN\User\mcp.json`

This User directory is the parent of `globalStorage/state.vscdb` already used by `traework_models.rs`.

Local User dir exists; `mcp.json` not created until the user adds a server.

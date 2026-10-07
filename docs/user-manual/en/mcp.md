# MCP management

## Add a service

Open 「MCP 管理 → 添加 MCP」. Enter a unique ID, name, and optional description and tags.

In 「快速配置」 (Quick configuration), fill in the transport's fields:

- `stdio`: command, one argument per line, optional working directory, and environment variables as `KEY=VALUE`.
- HTTP / SSE: connection URL and required headers as `Name: Value` or `Name=Value`.

Alternatively, enter a single service configuration in 「JSON 编辑」. Review software assignments before saving. Use 「编辑」 in an existing service's details to change it.

## Discover and import

Search the curated catalog in 「发现」. Check its documentation, authentication and runtime requirements, select a target, fill in the requested parameters and install. Prepare Node.js / npx or uv / uvx when required.

「导入现有」 (Import existing) reads MCP configurations from installed software. Check the imported count and content under 「已安装」.

## Assign and maintain

Select a service and enable or disable target software in the assignment area. Refresh that software's MCP list and try a call. After saving configuration, check the runtime, credentials and remote service availability.

「重新配置」 overwrites existing configuration; review manual edits before confirming. 「删除」 removes the service from management and enabled software. Environment variables and headers can contain secrets; redact them before sharing diagnostic material.

## Bulk preview and partial failures

Use the header 「批量分配」, filter/select services, choose one target and enable/disable intent, then preview and confirm. Completion requires authoritative readback, not acknowledgement alone. Preview again after drift. A failure may have partial writes: refresh current state before selecting unfinished rows for a new preview. Assignment success does not prove a remote service or target-app call succeeded.

[Back to manual](README.md)

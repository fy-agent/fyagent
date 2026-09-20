# Model management

Open 「模型管理」 and select the target under 「选择应用」. Follow the configuration offered for that software. Add TRAE Work CN's third-party models inside TRAE Work CN, then return to view them in FyAgent.

## Enter and save settings

1. Enter a configuration name where offered, service URL, API Key and model ID. Use an HTTP(S) URL; put the key only in its API Key field.
2. Use 「拉取模型」 (Fetch models), or enter IDs manually. WorkBuddy and OpenCode accept multiple IDs; review the pending list.
3. Click 「测试连通」 (Test connectivity), select a model and 「开始测试」. This sends a request and may incur a small amount of usage.
4. Click 「保存并设为当前配置」 (Save and make current) or 「保存并应用」 (Save and apply). Review file changes and backup information, then confirm.
5. Check the current configuration and refresh, reopen or start a new session in the target software as instructed.

Claude Code's service URL usually omits the trailing `/v1` to avoid a duplicated path; confirm with the service's documentation. For a local service without authentication, use 「不使用 API Key」 when offered. Choose Codex WebSocket transport and image-generation extension settings according to service compatibility.

Confirm the target before overwriting or deleting models. If settings changed elsewhere or a save needs confirmation, reload the current settings before continuing.

## Use an account subscription

Under 「使用账号订阅（实验性）」, select a saved ChatGPT or Grok account and choose or enter a supported model ID. Grok example options are subject to actual service availability.

For Codex, click 「保存 Codex 订阅配置」, then continue to accounts and authentication to preview and confirm the source switch. For other targets, confirm the file changes through the offered apply action. Keep FyAgent running in the background; fully quitting stops local forwarding. Actual availability and quota usage depend on the service response.

See [accounts](accounts.md) and [troubleshooting](troubleshooting.md).

[Back to manual](README.md)

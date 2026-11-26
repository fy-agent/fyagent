# Model management

Open 「模型管理」 and select a target under 「选择应用」, or use 「一键配置主流大模型」 and 「全端体检与克隆」 at the top to configure multiple clients together. `QoderWork CN` and `TRAE Work CN` only support vendor-provided official models; add third-party models inside those apps if offered.

## Configure a single client

1. Enter a configuration name where offered, service URL, API Key and model ID. Use an HTTP(S) URL; put the key only in its API Key field.
2. Use 「拉取模型」 (Fetch models), or enter IDs manually. WorkBuddy, OpenCode and Oh My Pi (`omp`) accept multiple IDs; review the pending list.
3. Click the inline 「测速」 button next to a saved model (or 「测试连通」) to verify connectivity. Probing sends a minimal request and never deletes or modifies saved models even on HTTP 404.
4. Click 「保存并设为当前配置」 or 「保存并应用」. Review the target file list and backup notes before confirming.
5. Verify the live summary and refresh, reopen or start a new session in the target software as instructed (when Codex is running, a top banner offers one-click CLI daemon restart or desktop reload guidance).

Claude Code's service URL usually omits the trailing `/v1` to avoid a duplicated path; confirm with the service's documentation. For a local service without authentication, use 「不使用 API Key」 when offered. Choose Codex WebSocket transport and image-generation extension settings according to service compatibility.

## One-click batch setup, cross-client clone, key rotation, and custom templates

- **Batch preset setup**: Pick a built-in provider (DeepSeek, Kimi, Zhipu GLM, Aliyun DashScope, Volcengine Ark, MiniMax, SiliconFlow, OpenRouter) or a saved custom gateway template, paste one API Key, choose target clients, preview affected file paths, and apply.
- **Cross-client health overview**: Inspect all six configurable clients together, probe them concurrently, clone one working client to others, rotate an API Key in place across clients sharing the same host, or save a custom relay as a reusable template (templates never store API Keys).
- **Per-client isolation and rollback**: Each client is backed up before writing and validated afterward. If one client fails, only that client is rolled back and reported clearly, with a one-click retry for failed targets.

## Which files and fields are modified, and what is never touched

| Client           | Target files                                         | Updated fields only                                                        | Never modified                                                                              |
| ---------------- | ---------------------------------------------------- | -------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| WorkBuddy        | `~/.workbuddy/models.json`                           | Selected model entries' base URL, API Key, and model ID                    | Other custom models, chat history, MCP, Skills, official subscription sign-in               |
| Codex            | `~/.codex/config.toml`, `~/.codex/auth.json`         | Active custom provider URL, wire protocol, model ID, and `OPENAI_API_KEY`  | Session history, `AGENTS.md`, MCP list, Skills, unrelated config sections                   |
| Claude Code      | `~/.claude/settings.json`, `~/.claude.json`          | `env` keys `ANTHROPIC_BASE_URL`, `ANTHROPIC_AUTH_TOKEN`, `ANTHROPIC_MODEL` | Session history, `CLAUDE.md`, MCP servers, Skills, permissions, and unrelated user settings |
| OpenCode         | `~/.config/opencode/opencode.json`                   | Selected custom provider's `baseURL`, `apiKey`, and model list             | Other providers, UI preferences, MCP servers, and unrelated custom fields                   |
| Oh My Pi (`omp`) | `~/.omp/agent/models.yml`, `~/.omp/agent/config.yml` | Managed provider URL, API dialect, key, and default model selector         | YAML comments, other providers, unknown fields, and private `agent.db`                      |
| Grok Build       | `~/.grok/config.toml`                                | Selected model URL, display name, protocol, and API Key                    | Session history, prompts, MCP, Skills, and unrelated settings                               |

**Schema upgrade backup and downgrade protection**: Before migrating the local database schema, FyAgent creates a validated binary backup automatically. If you launch an older FyAgent build against a newer database, startup stops with a clear prompt to upgrade FyAgent instead of failing silently or corrupting data.

## Use an account subscription

Under 「使用账号订阅（实验性）」, select a saved ChatGPT or Grok account and choose or enter a supported model ID. Grok example options are subject to actual service availability.

For Codex, click 「保存 Codex 订阅配置」, then continue to accounts and authentication to preview and confirm the source switch. For other targets, confirm the file changes through the offered apply action. Keep FyAgent running in the background; fully quitting stops local forwarding. Actual availability and quota usage depend on the service response.

See [accounts](accounts.md) and [troubleshooting](troubleshooting.md).

[Back to manual](README.md)

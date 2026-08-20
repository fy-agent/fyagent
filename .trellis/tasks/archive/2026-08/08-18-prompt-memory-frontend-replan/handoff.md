# 两台机器怎么分

把下面两份任务整段交给对应电脑，不要混着做。

| 电脑 | 文件 | 只做什么 |
| --- | --- | --- |
| 有真实 Agent 提示词的那台 | `handoff-prompts.md` | 只验收 `#/prompts` |
| Windows 开发机 | `handoff-windows.md` | 官方 `mise run check` + 三页桌面 |

共同前提：

- 分支 `cursor/prompt-memory-frontend-align-06e7`
- PR https://github.com/fy-agent/fyagent/pull/111
- 合入 `dev/laiyongjie`
- 不要在 `dev/laiyongjie` 上改，不要再 merge `dev/xk`
- 合并后先不要关这个 PR
- Node 用仓库 `.node-version`（24），不要用 22

```powershell
git fetch origin cursor/prompt-memory-frontend-align-06e7
git checkout cursor/prompt-memory-frontend-align-06e7
git pull origin cursor/prompt-memory-frontend-align-06e7
```

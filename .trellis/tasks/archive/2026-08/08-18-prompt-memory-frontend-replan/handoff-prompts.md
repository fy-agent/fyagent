# 提示词测试机任务

只测 `#/prompts`。不要跑 `mise run check`，那是 Windows 机的事。

拉代码：

```powershell
git fetch origin cursor/prompt-memory-frontend-align-06e7
git checkout cursor/prompt-memory-frontend-align-06e7
git pull origin cursor/prompt-memory-frontend-align-06e7
```

用本机编出来的 FyAgent 打开提示词页。目标：点开就能读正文，不用找分隔条，也不要弹出编辑窗口。

## 必做

1. 左轨七个应用都在：Claude、Codex、Gemini、Grok Build、OpenCode、OpenClaw、Hermes。默认 Claude。
2. 每个应用副文案是「N 条已启用」，不是「提示词库」。切到有启用项的应用，数字要变。
3. 点开一条：中间立刻是可编辑正文。不要出现「编辑」Dialog。不要先看到一长串 ID / 时间定义列表。
4. 常见窗口下不拉分隔条也能看到正文。名称、描述在正文下面。
5. 「当前使用的内容」默认折叠。展开后只读。
6. 「新建提示词」是空草稿，保存后不自动启用。
7. 「从文件导入」仍可用，但是次动作。
8. 列表选中和启用开关不是同一件事。一应用只能一条启用。已启用的不能直接删，先停用。
9. 未保存时切应用、切条目、切到记忆页，必须出现放弃确认，不能是浏览器 `window.confirm`。
10. 选中 B，搜索只能搜到 A：左边只剩 A，右边仍是 B，并出现「当前编辑的提示词不在搜索结果中」。
11. 选中 B，搜索完全无结果：不要整页变成「没有匹配的提示词」空态，右边仍是 B。
12. 浏览器预览打开提示词页应是 native-only，不要演示数据。

## 不要测

提示词市场、跨应用同步、Claude Desktop。不在合同里。

## 回报

按条写「过 / 不过」。不过就写：窗口大小、操作序列、期望 vs 实际。结果回 PR #111，不要关 PR。

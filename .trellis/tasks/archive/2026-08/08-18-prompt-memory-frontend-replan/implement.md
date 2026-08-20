# 提示词与记忆前端重规划 — 执行计划

## 0. Git

- [x] 确认当前分支是 `cursor/prompt-memory-frontend-align-06e7`，且 `origin/dev/laiyongjie` 是祖先。
- [x] 确认没有 upstream 指向 `origin/dev/laiyongjie`。push 只用 `git push -u origin cursor/prompt-memory-frontend-align-06e7`。
- [x] PR base 设为 `dev/laiyongjie`。

## 1. Spec

- [x] 在 `v2-prompts-memory.md` 增加目录轨、就地编辑、折叠 live file、记忆元数据进编辑头。
- [x] 不改 port 签名、资源 ID、失败矩阵。
- [x] 记下 OpenClaw 图标回退。

## 2. 提示词页

- [x] 用 `CatalogMasterDetail` 替换 `<select>`；七个应用，Claude 默认。
- [x] 搜索移进 workspace。
- [x] 中间栏就地编辑；去掉编辑 Dialog。
- [x] live file 改为默认折叠的检视。
- [x] 保留导入、启用、删除、写锁、回读警告、脏确认。
- [x] 更新单测与 browser 用例：不再假设三栏常驻和 toolbar `<select>`。

## 3. 记忆页

- [x] 长期：OpenClaw / Hermes 分组轨；去掉「记忆信息」第三栏。
- [x] Hermes 开关、字符上限、可复制路径、打开目录进编辑头。
- [x] 每日：搜索 + 列表 + 编辑两栏。
- [x] 页头文案改为准确范围。
- [x] 保留 missing create-on-save、写锁、回读警告、脏确认。
- [x] 更新单测与 browser 用例。

## 4. 验证

```bash
mise run lint:v2
mise run typecheck:v2
mise run test:v2
mise run test:v2:browser
mise run build:renderer
mise run format:check
git diff --check
```

- [x] Linux 上四档视口 Playwright 已过；Windows 125%/150% 与完整 `mise run check` 见 `handoff.md`。
- [x] `git diff origin/dev/laiyongjie` 不含 MCP Catalog、安装器、`dev/xk` 未合内容。
- [x] 不出现对 `origin/dev/laiyongjie` 的 push。

人工验收清单：`handoff.md` 分发；`handoff-prompts.md` 给提示词测试机；`handoff-windows.md` 给 Windows 官方 check + 三页桌面。

## 回滚

还原两页、测试和 spec IA 段。shared 底盘保持基线。

# Implementation and validation plan

## Plan

- [x] 枚举 2026-09-16 四个行为提交、最终 diff、归档任务、当前 SPEC 和测试 owner。
- [x] 核对 FDE 提示词/MCP 的封闭集合、静态/原生边界、secret/权限与精确回归；只
  补稳定契约缺口。
- [x] 核对 macOS helper driver 的当前产物优先级、遗留产物拒绝和封印测试。
- [x] 沿 Managed Auth/Provider/Proxy/Agent/Renderer 完整数据流核对命令、DTO、错误、
  refresh/401、补偿、route observation 与 overview parser。
- [x] 按 owner 拆分后端订阅代理契约、泛化前端订阅契约文件名，更新索引、相邻引用
  和任务 context；移除重复 owner。
- [x] 未发现产品实现偏差；保持产品代码不变，只增加精确集合回归。
- [x] 运行格式、链接/契约、聚焦测试、task validate 和完整归档前门禁；复核证据边界。
- [x] 复核最终 diff 与工作树，提交工作变更，并准备按 finish-work 归档与记录会话。
- [x] 按 merge-governance 确定归档后的 exact-head handoff：归档后契约检查、base drift、
  推送/PR、exact-head auto-merge、PR/merge-group CI 和最终 main 读回；远端执行证据由
  GitHub 与最终交付记录，不伪装成归档前证据。

## Focused validation candidates

最终命令以实际写集为准，至少包括：

```bash
git diff --check
mise run format:check
mise run check:contracts
python ./.trellis/scripts/task.py validate \
  .trellis/tasks/09-16-review-todays-specs-and-merge
```

若修改 Renderer/类型/测试：

```bash
mise run typecheck
mise run test:unit -- <affected renderer tests>
```

若修改 Rust/脚本行为：

```bash
mise run rust:fmt:check
mise run rust:test -- <affected filters>
mise run test:unit -- tests/miseTaskContract.test.ts
```

归档前完整门禁：

```bash
TRELLIS_CONTEXT_ID=fyagent-review-20260916-specs \
  mise run check:prearchive \
  --exclude-active-task .trellis/tasks/09-16-review-todays-specs-and-merge
```

归档后：

```bash
mise run check:contracts
```

## Review gates

- 一个稳定行为只有一个语义 owner；索引和相邻文档只能链接/描述交界。
- 所有命令、DTO、错误码、env/字段名、目标差异和 parser 不变量可由代码/测试直接定位。
- SPEC 文件不能因 context 注入大小而截断；不得提高上限掩盖 owner 过大。
- 不把历史浏览器/真实 loopback/当前 macOS 宿主证据扩张成未执行的 HIL 或真实账号结论。
- 推送后 PR head 必须等于本地最终复核 head；新增提交后重新设置 exact-head guard。

## Review outcome

- FDE 提示词从“六类/若干场景”收敛为精确 6 个分类 ID、固定顺序、每类 5 个和精确
  搜索字段；MCP 从“相关国内条目”收敛为有序 20-ID 集合。两处都增加精确单测。
- macOS driver 实现与测试已经正确优先当前 SwiftPM `release/` 产物；SPEC 补上双产物
  同时存在时的错误矩阵和 Wrong/Correct，不改脚本行为。
- 新增后端 `managed-account-proxy.md`，将账号准入、Provider 身份、目标差异和 overview
  投影从 core 拆出。`managed-auth.md` 从 29988 bytes 降到 25099 bytes；所有 required
  owner 均低于默认 32768-byte 注入上限。
- 前端当前 owner 改为 `managed-account-subscriptions.md`；旧文件只做历史路由，归档
  任务引用仍可解析。Proxy Runtime 和 Local Proxy 只保留各自 listener/HTTP owner。
- 工作提交：`a04f68ff docs(spec): refine today's behavior contracts`。

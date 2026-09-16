# Design

## Review boundary

评审按四条独立证据链进行，并在最终 PR 前做一次跨链集成复核：

```text
FDE static catalogue -> Renderer browse/draft -> existing native Port
macOS build driver -> current output tree -> copied helper -> seal/test contract
Managed account -> credential lineage -> Provider binding -> ProxyService
  -> Agent target/readback -> request transport -> overview projection/parser
Git task lifecycle -> local gates -> exact PR head -> merge-group -> main readback
```

代码和测试决定当前事实，归档任务提供设计理由与证据边界，SPEC 只保留以后实现者
仍需执行的稳定契约。相邻文档只描述交界，不复制完整状态机。

## SPEC owner convergence

- FDE 保留 `prompt-presets.md` 与 `mcp-fde-catalog.md` 两个专属 owner；
  `prompts-memory.md` / `mcp.md` 只拥有原生 CRUD、Port 和通用页面行为。补充内容只
  针对封闭分类/成员、精确构造或遗漏的失败矩阵，不重复目录正文。
- macOS helper 继续由 `backend/macos-system-commit.md` 拥有。脚本 driver 与候选路径
  顺序写成单一契约；封印测试断言脚本文本中的实际顺序而不是只检查路径存在。
- 后端 Managed Auth core 只保留 identity/credential、SecretRef、迁移、默认/删除、
  refresh 与 access-material 解析。账号绑定到本机 Agent、Provider 身份、目标激活、
  route observation 和 overview connection 投影若在 core 中混杂，则迁到新的聚焦
  订阅代理 owner。`proxy-runtime.md` 仍拥有 listener 生命周期/补偿，
  `local-proxy-pipeline.md` 仍拥有请求转换、重试、SSE 和 usage。
- 前端通用订阅工作流使用语义名称 `managed-account-subscriptions.md`；历史
  `grok-subscription.md` 不再作为当前 owner。`models.md` 只保留 Models 页面与 Port
  交界，`managed-auth.md` 只保留 overview parser/账号页语义。
- 所有新 owner 均使用 Trellis 七段 code-spec 结构，并由 backend/frontend index
  路由。拆分通过移动现有稳定内容完成，不复制两套要求。

## Implementation mismatch policy

默认只修改 SPEC、索引、任务上下文与必要链接。发现以下任一情况时才修改代码：

1. SPEC 描述的是产品必须保持的现有边界，但实现/回归没有真正执行；
2. 完整 DTO、错误或补偿路径会违反严格 parser、secret 或用户数据安全；
3. 当前测试只断言表面字符串，不能阻止同类回归。

代码修复必须落在行为 owner，并附聚焦回归；不在 caller 增加掩盖根因的 fallback。

## Validation and evidence

- 先以结构搜索、类型检查和聚焦单测证明签名/引用；SPEC-only 更名仍运行链接、格式、
  task context 与 repository contract 检查。
- 完整 `check:prearchive` 是归档前权威本地门禁；浏览器矩阵只在产品/测试交互实际
  改变时重跑，否则沿用今日实现任务的历史证据但不把它冒充本次执行。
- PR CI 证明推送 head 在 GitHub runner 上通过；Merge Queue 的 merge-group 才是
  最新 base 集成证据。任一新修复提交都会废止旧 exact-head handoff。

## Git and rollback

不为了包含已经合并的历史 merge commit 重写今日提交；提交前重新读取
`origin/main...HEAD` 和 GitHub base drift。若无冲突，Merge Queue 负责最新 base
集成；若本地检查或 GitHub 明确要求同步，则用非破坏性 merge/rebase 路径并重新
验证精确 head。

SPEC 拆分/重命名可通过单个工作提交回滚，不改变持久化数据或 IPC。若同时发现行为
缺陷，修复与回归单独成提交，便于独立回滚且不丢失文档收敛。

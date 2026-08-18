# Prompt / Memory 前端设计入口

## 1. 权威文档

本文件是设计导航与冻结摘要，不再重复维护第二套字段和状态转换。

按以下顺序读取：

1. `prd.md`：产品范围、用户任务和验收条件。
2. `technical-design-overview.md`：技术架构、三方案比较、模块边界、未来 port、兼容/回滚。
3. `detailed-design-overview.md`：文件 owner、TypeScript 类型、状态转换、交互、测试与证据。
4. `execution-plan.md`：阶段门禁、三线路派发、模块单测、集成、提交和推送顺序。
5. `reviews/*.md`：产品、架构和详细设计静态评审记录。

research 只提供事实依据；历史 `*-v4-implementation-notes.md` 不是当前产品模型。

## 2. 采用方案

采用 `页面内显式 saved baseline + 窄共享目标合同`：

- Prompt 与 Memory 各自在页面内持有 saved snapshot、draft、baseline 和领域转换。
- 不建立全局 store、service/container、空 adapter 或通用表单框架。
- 两页只共享 `agentTargets` 的工具/实例/目标/资格/canonical Prompt resource 合同。
- 两页各自在自身 `Page.tsx` 使用一个 React Router `useBlocker`；不修改 router、navigation、PrimaryNav 或 AppShell。
- prototype 与 durable backend result 分离；本轮不实现 data source port 或 native 调用。
- standalone 从 `dist/index.html` 解析当前 production entry graph，不再猜最大文件。

被排除的方案：

1. 只修文案、继续沿用混合 state：不能关闭 saved baseline、provenance、逐目标 preview task 缺口。
2. 全局 store / Shell 级 guard：抽象过早，扩大与其他前端分支的冲突。

完整比较与取舍见 `technical-design-overview.md`。

## 3. 冻结产品合同

### Prompt

- 9 条 grounded 内置规则；2 条默认启用。
- 多条规则同时启用，互不关闭。
- 每条规则零到多个目标；启用必须有至少一个最后保存的目标。
- 7 个唯一 Prompt 资源覆盖 8 个实例；OpenClaw main + utility 共用一个资源。
- Gemini/OpenCode 显示“启用时创建”。
- 新建项首次保存前始终 dirty；放弃后不留空 committed item。
- dirty 覆盖条目切换、新建和 SPA pathname 离开。

### Memory

- 分类固定为长期记忆、每日记录、会话记录。
- Daily/Session 本轮是只读来源，只能提炼，不能直接保存原始记录。
- 提炼生成未保存长期草稿，保留 source item/target/tool/path/time/summary。
- 可编辑长期条目的标题、正文和同步目标共同组成 saved revision。
- 只有 Claude rule bridge、两个 OpenClaw workspace、Hermes 四个 verified target group 可选。
- 点击同步只生成逐目标 `pending` preview task，durable state 固定 `not-run`。
- 保存新 revision 清空旧 preview tasks；放弃 draft 保留原 revision/tasks。
- 路径状态明确为已存在、未发现或前端草稿。

## 4. 架构边界

保留：

- V2 Hash Router、六路导航、默认 `#/models`。
- CC Switch/FyAgent 现有框架、V2 Shell、窗口端口和 Tauri 隔离。
- `app -> pages/widgets/shared` 的依赖方向。
- 深蓝 Developer Tool、现有 `--fy-*` token 和三栏工作区。

不修改：

- `src-tauri/**`、数据库、Rust command、真实 Agent 文件。
- Agent 目录、模型、Skills、MCP 页面。
- `navigation.ts`、`router.tsx`、`widgets/app-shell/**`。
- 无关图片目录。

## 5. 共享目标合同摘要

- `promptCanonicalResourceKey` 只用于 Prompt instruction resource 去重。
- Memory 本轮使用 4 个 adapter/scope 目标组；不借 Prompt path 推断 `MEMORY.md` / `USER.md` 文件身份。
- `memorySyncEligibility` 是 `source-only / verified-rule-bridge / verified-native`。
- Memory writable IDs 从资格字段派生。
- invalid target lookup 返回 `undefined`，不静默回退 Codex。
- 未来真实 path normalization、realpath、symlink/case 处理属于后端扫描层。

## 6. Prototype 真实性

- 根节点和用户可见文案都明确“前端原型 · 未读取或写入本机文件”。
- seed 只保留匿名化结构，不包含私人正文、凭据或用户名绝对路径。
- 保存只更新前端 preview；扫描是模拟结果；同步只生成待执行任务。
- 不使用没有外部回读证据的 durable“已同步”。

## 7. 评审状态

- 产品设计评审：`DESIGN_REVIEW=PASS`；初审 1 个 P0、5 个 P1 已在设计层关闭，并由本轮 unit/browser 运行验收验证。
- 技术架构评审：`ARCHITECTURE_REVIEW=PASS`，P0/P1=0；P2/P3 已在详细设计锁定。
- 详细设计评审：`DETAILED_DESIGN_REVIEW=PASS`；原 3 个 P1 已关闭，P2/P3 已进入模块与最终验收。
- 设计冻结：`DESIGN_FREEZE=2026-08-12`。
- 最终质量复核：`FINAL_TRELLIS_CHECK=PASS`；证据见 `research/verification.md`。

评审阶段只做静态阅读；没有执行 lint、typecheck、unit/browser test、build、dev server 或截图。

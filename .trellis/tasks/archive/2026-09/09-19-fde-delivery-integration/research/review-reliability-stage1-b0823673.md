# 配置线阶段 1 独立审查

范围：固定提交 `b0823673`，基线 `fd1c8446`，主题 `separate configuration eligibility from install trust`。全部产品与测试依据取自 `git show` / `git diff` 的固定提交；未读取变化中的工作树代码，未修改产品，未运行依赖未提交代码的测试。

## 结论

**R7 核心拆分静态审查无阻断；有 1 个必须修的浏览器 fixture 回归。** 新字段只决定配置入口，没有被用作安装、更新或配置写入授权。前后端 v5 字段与来源枚举匹配。当前证据不足以宣称原机器上的 R7 已复现并修复，也不足以宣称本提交的新增 Rust 测试已通过。

## 必须修

后续状态（root 回报，2026-09-19）：本阶段已集成到 control `18401fe1`，root 已删除 TRAE fixture 的多余字段，原生 `configuration_` 定向检查进行中。本报告保留原始发现；该修复未在本次 R5 审查中另行复跑。

### [P2] 删掉 TRAE 模型列表 fixture 中误加的 `editable`

- 改动位置：`tests/browser/support/features.ts:1225-1230`，尤其 `:1228`。
- 依据：提交给 `get_traework_model_ids` 的返回值新增 `editable: true`；同一固定提交的 `src/shared/platform/tauri/feature-ports/qoderTrae.ts:428-436` 使用 `hasExactKeys(value, ["modelIds", "revision", "truncated"])`。因此经过真实 port 的 rich browser fixture 读取必定抛出 `TRAE model list is unavailable`，无法显示已有 TRAE 模型。它不是 OpenCode provider 的可编辑字段，不能以 R1 fixture 补齐解释。
- 最小补救：仅移除该 TRAE 返回值中的 `editable`，保留本次 readiness v5 fixture 更新；不放松生产解析器、不新增 TRAE DTO 字段。
- 验证：在修复后的稳定阶段用既有 TRAE 浏览器读取场景或直接 port fixture 检查确认模型列表正常即可；无需为此跑全库。

## 核心行为核对

1. **导航资格确实从安装来源分离。** `src-tauri/src/agent_install/mod.rs:84-95、98-132、258` 先从 CLI 观察生成资格，再调用 inventory overlay。`:135-169` 仍将 multiple/unknown 的安装状态和版本降为未知，但不抹掉已检测/可运行 CLI 事实。`src/pages/agents/agentDirectoryScanProjection.ts:47-70` 只使用 native eligibility 决定配置入口；`AgentDirectory.tsx:86-108` 显示来源不确定，而非宣称安装可信。
2. **失败或没有观察不会制造正面资格。** native mapping 将 NotInstalled/Unknown/Unavailable 分别映射非 eligible 状态及 evidence=none。新增 `missing_or_failed_cli_observation_never_grants_configuration` 覆盖没有观察、缺失及 unavailable，与 single/multiple/unknown inventory 交叉。正面用例同时覆盖 runnable/detected 两类 CLI。
3. **native 与前端 wire 一致。** Rust `types.rs` 与 TS `agent-install-readiness.ts` 同时升到 v5；TS `:461-564` 要求精确字段、来源匹配及有效 state/evidence 组合，缺字段、旧 v4、多余字段和错误来源均拒绝。该变更要求 native/renderer 同时集成，提交说明对此已明确。
4. **安装/更新权限未改成由 eligibility 授予。** `src-tauri/src/agent_install/mod.rs:560-566` 仍先经过 `admit_action` 与 `validate_action_target`；`src/shared/features/agent-lifecycle-capabilities.ts:43-58` 仍依据安装与更新状态、allowedActions 判断。新的资格字段没有进入这些权限判定。
5. **配置页可读，但本提交没有新增“只读模式”。** 配置进入后仍用已有 `AgentModelsSection` 读取模型摘要，Skills/MCP 保留已有显式分配动作（`AgentAssignmentSections.tsx:50-67、203-216`）。这符合资格只负责导航、资源 owner 负责操作的划分；不能把本提交描述为将整个配置页锁成只读。全局搜索新资格字段后，产品侧消费者仅为目录/解析与 native readiness，没有配置 writer 消费该字段。
6. **没有向前端输出工程注释。** 新文案只解释检测结果、能否进入配置和安装目标不确定性；未显示 revision、内部接口、审核状态等工程细节。

## 测试与验收边界

- 新增前端 `Page.test.tsx:755-792` 确认 multiple/unknown 下“进行配置”可用、安装/更新按钮不出现、点击后进入配置且不触发 startAction；它证明页面导航，不证明真实 native 配置读取或写入隔离。
- 新 native tests 覆盖 R7 已定位的纯投影分支，能防止 inventory 再次抹去正面 CLI 观察；它们使用构造的 CliObservation，没有经过真实 Tooling 探测。因此不能证明用户原机器实际触发的就是此分支。
- 固定提交内 `research/readiness-implementation.md` 记载 renderer 13 文件 / 158 用例及 typecheck 通过；这是实现线记录，本次没有重跑。该文件明确 Rust 编译被当时另一项 MCP 修改阻断，新 Rust 测试未执行；root 应在稳定集成版本跑一次既定 `agent_install::tests` 定向检查并确认命中新测试。
- 不要求为了本审查额外制作平台框架、全库测试或真实安装。最终若要报告“原机器 R7 已修复”，须另有隔离原生运行回读；当前准确结论是源码修复及前端 fixture 证据。

以上路径均相对于固定提交所在仓库 `~/.codex/worktrees/fyagent-fde-reliability/fyagent`。本报告仅拥有并修改当前文件。

# 实施计划

## Phase 1：研究与任务合同

- [x] 确认 Registry access-mask 根因及 Microsoft 合同。
- [x] 评审 cargo-xwin/xwin 与拒绝替代方案。
- [x] 复核 live first-party Qoder/TRAE/WorkBuddy metadata schema。
- [x] 扫描仓库复用 owner，确认不新增 scanner/updater/animation state machine。

## Phase 2：Windows inventory

- [x] 增加 query+enumerate 的只读 inventory parent capability。
- [x] shell-user/machine parent 复用，child 保持 query-only。
- [x] 冻结 complete/no-candidate 与 incomplete 投影。

## Phase 3：AI 软件安装与更新

- [x] QoderWork/TRAE Work/WorkBuddy Desktop policy 启用 update。
- [x] 复用既有 source、download、helper/DMG transaction 与 post-readback。
- [x] 抽取 `desktop_allowed_actions` 并覆盖 readiness/target-selection tests。
- [x] 确认 frontend 只按 backend `allowedActions` + `update_available` 执行一键更新。

## Phase 4：macOS Windows-MSVC 诊断

- [x] 实现 fixed plan、strict preflight 与 default-no cargo-xwin runner。
- [x] 增加 bootstrap read-only non-failing advisory。
- [x] 覆盖缺失工具、错误 host/version、override、fixed argv、无副作用与 DAG 测试。

## Phase 5：前端视觉稳定性

- [x] SelectionLens 增加默认兼容的 geometry mode。
- [x] SideNavigation 使用 position-only 并清除重复 active material。
- [x] 更新 unit/browser/static tests。

## Phase 6：SPEC、评审与验证

- [x] 更新 development-environment/task-runner/windows-runtime/external-agent/v2-agent-models/v2-shell owning specs。
- [x] 评审权限、source、target、helper 与 post-readback 安全边界。
- [x] 评审复用、跨平台 policy、默认开发体验、UI/accessibility 与证据声明。
- [x] focused gates 与完整 prearchive gate。

## Phase 7：归档

- [x] 提交最终实现与 SPEC。
- [x] Trellis archive。
- [x] 写 journal；归档后审计产生的 SPEC 修正与最终 full check 已补充记录。

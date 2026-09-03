# 收敛 Auth 集成、故障恢复、Spec 与原生验收

## Goal

收敛所有 Auth 入口、完成故障恢复和跨平台验收，并更新 Spec、归档父子任务。

## Background

- 本任务是父任务 `.trellis/tasks/09-03-unified-agent-auth-management` 的可独立验收切片。
- 产品、协议、复用、许可证和安全基线以父任务 `prd.md`、`design.md` 与 `research/` 为准；发现外部事实变化时先更新 research，不在代码中猜测。

## Requirements

- 把 V1 Auth Center 变成统一 service 的兼容 UI 或迁移入口，消除独立轮询/状态 owner。
- 统一 Proxy account binding、Agent Auth observation、Provider forms 和中央页面。
- 完成 destructive action impact preview、restart queue、external-change recovery、migration blocked UX。
- 安全、许可证、依赖、日志、并发、可访问性和性能复审。
- 完成 macOS/Windows native HIL、Spec 更新、提交、任务归档和工作树清洁。

## Acceptance Criteria

- [ ] 只有一个 Managed Auth backend authority 和一个 V2 主体验。
- [ ] V1、Agent、Proxy、Provider 入口均引用同一账号/连接状态。
- [ ] 所有自动化检查与必要 native HIL 通过，未验证项保持 fail-closed。
- [ ] Spec 与最终实现一致，全部子任务及父任务归档。
- [ ] 最终 Git 工作树 clean。

## Out of Scope

- 没有证据的 Linux 产品支持或新 Provider。
- 跳过 native HIL 后把 mock 结果描述为完整交付。

## Open Questions

无阻断性产品问题；技术不确定项必须通过第一方源码、官方文档、当前仓库和 native HIL 收敛。

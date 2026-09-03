# 建立 V2 账号与认证主体验及严格前端合同

## Goal

以前端可理解、可恢复、证据正确为第一优先级，建立 V2 账号与认证主体验和严格传输合同。

## Background

- 本任务是父任务 `.trellis/tasks/09-03-unified-agent-auth-management` 的可独立验收切片。
- 产品、协议、复用、许可证和安全基线以父任务 `prd.md`、`design.md` 与 `research/` 为准；发现外部事实变化时先更新 research，不在代码中猜测。

## Requirements

- 新增 V2 `/auth` 主路由、导航入口和持久页面生命周期。
- 实现账号列表、账号详情、软件连接、当前请求来源、登录会话与危险操作预览的前端信息架构。
- 扩展或替换现有 Agent Auth DTO/Port，使前端只消费严格解析后的账号、连接和会话快照。
- 建立 browser fixture 与可访问性/响应式测试；原生写入未就绪时明确显示不可用而非模拟成功。

## Acceptance Criteria

- [ ] `/auth` 可从主导航进入并支持直接链接、返回 Agent 上下文和页面 keep-alive。
- [ ] 账号、软件连接、当前请求来源三类状态在视觉和文案上不可混淆。
- [ ] 登录/刷新/重登/断开/删除/重启等待等状态均有可操作、可恢复且不夸大的 UI。
- [ ] 所有跨层响应经 exact-key 闭集解析；token、路径、命令、raw error 不进入 React 状态或 DOM。
- [ ] 键盘、焦点、ARIA、窄窗和 reduced-motion 测试通过。

## Out of Scope

- 真实 OAuth token 交换、OS 凭据库写入和 vendor 配置文件修改。
- 复制 V1 Auth Center 组件到 V2。

## Open Questions

无阻断性产品问题；技术不确定项必须通过第一方源码、官方文档、当前仓库和 native HIL 收敛。

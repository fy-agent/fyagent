# 执行计划

- [x] 读取上轮结果、源码及三类 GitHub open 告警；运行初始依赖/密钥扫描。
- [x] 核实官方修复/API、写入审查资料，激活任务。
- [x] 定向更新依赖；复扫并核对跨平台依赖图和残余风险。
- [x] 复用成熟 parser/标准 URL，修复 HTML/URL/平台扫描器问题，补充专项回归。
- [x] 审查日志/原生告警，修复真实风险，保留测试与上游语义。
- [x] 真实 TS 依赖图扫描、修复架构边界、加入防回退门禁。
- [x] 集成评审、补扫、更新所属 SPEC 和平台身份摘要。
- [x] 完成所有受影响门禁和完整 prearchive（最终源码与 CI 分类检查，退出 0）。
- [ ] 工作提交、归档、日志及无排除 postarchive 校验。

## 验证入口

`mise run typecheck`、`mise run typecheck:v2`、`mise run lint:v2`、`mise run test:unit`、`mise run test:v2`、`mise run check:backend`、`mise run build:renderer`、`mise run test:v2:browser`。

最终：`TRELLIS_CONTEXT_ID=fyagent-security-architecture-round2 mise run check:prearchive --exclude-active-task .trellis/tasks/09-05-security-architecture-round2`；归档后 `mise run check:contracts`。

安全工具的网络/数据库新鲜度和版本单独记录；离线行为回归不声称替代联网公告扫描或跨平台原生证据。

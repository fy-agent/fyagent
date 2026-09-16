# 复核今日变更的 SPEC 并完成合并

## Goal

复核 `dev/laiyongjie` 在 2026-09-16 新增的 FDE 提示词/MCP 预设、macOS
helper 产物选择、托管账号代理复用及空闲代理概览修复，使实现、测试与 Trellis
SPEC 的可执行契约一致；完成本地质量门禁、任务归档、PR、PR/Merge Queue CI 和
最终 `main` 合并读回。

## Background

- 当前分支相对 `origin/dev/laiyongjie` 领先十个提交，其中四个是行为提交，其余为
  已归档任务和会话记录。`origin/main` 额外包含前两轮 PR 的 merge commit。
- FDE 变更是 Renderer 静态目录与现有原生写入链路的组合；macOS 修复是发布脚本
  对当前 SwiftPM 产物与遗留 Xcode 产物的选择；托管账号代理横跨凭据、Provider、
  loopback 代理、Agent 配置和 Renderer；最后一个修复收紧完整 overview DTO。
- 原实现任务已经归档。本任务评审它们形成的当前稳定行为，不把一次性运行记录、
  提交 SHA、测试数量或 CI run ID 写入 SPEC。
- 今日已经修改十三个 SPEC 文件。后端 `managed-auth.md` 同时承载核心凭据、订阅
  绑定、刷新与代理观察，前端通用订阅契约仍使用 `grok-subscription.md` 的历史
  文件名；需要按 owner、上下文注入大小和后续可维护性重新判断拆分/重命名。

## Requirements

- R1：逐项核对今日四个行为提交及其代码、测试、归档任务和相关 SPEC，确认稳定
  行为均有可执行 owner；SPEC 只保留签名、契约、错误矩阵、案例和测试断言，不
  复制一次性验证证据。
- R2：FDE 提示词预设必须明确静态目录与原生提示词库的边界、封闭分类/字段、搜索
  交集、零写预览、脏草稿确认、独立禁用副本和 128-bit 随机 ID。FDE MCP 契约
  必须明确完整筛选成员、命令/env/敏感字段、权限风险和现有 native mutation owner。
- R3：macOS helper 契约必须准确描述当前 driver 的产物路径优先级：使用本次构建
  对应的 SwiftPM `release/` 产物，不得因遗留 `apple/Products` 存在而复制旧二进制；
  相关封印/脚本测试必须锁定此顺序。
- R4：托管订阅代理必须拥有聚焦契约，覆盖命令/DTO、显式账号与凭据准入、稳定
  Provider 身份、Claude/Grok 即时激活与 Codex Change Plan、上游 secret 不落入
  Agent 配置、刷新/401 单次重放边界、listener/config 观察和失败补偿。核心 vault、
  listener 生命周期与请求转换各保留一个 owner，不建立第二套状态机。
- R5：完整 Managed Auth overview 必须始终可被严格 Renderer parser 接受。
  `requestMode=none` 时 provider label 必须为空；`connectedConsumerCount` 按仍指向
  账号的唯一 consumer 计数，包括已命名但当前未路由的 `fyagent_proxy` 槽位。
- R6：将已经泛化的前端订阅契约移出 xAI/Grok 历史命名，并更新所有索引、相邻
  owner、任务上下文和仓库引用；如后端 owner 仍混杂，则拆出聚焦文件而不是扩大
  注入上限。不得留下失效链接或两套互相冲突的要求。
- R7：若评审发现实现与稳定契约不一致，必须在根因 owner 处做最小修复并添加回归；
  不为纯文档整洁扩大产品范围。执行聚焦检查、任务 context 校验和完整归档前门禁，
  明确 mock/browser/本机检查的证据边界。
- R8：完成任务归档和归档后契约检查；复核最终差异、工作树、base drift 与精确
  head 后推送分支、创建 PR，并跟进 PR 与 Merge Queue `CI / Required`。失败时在
  同一分支最小修复、重跑适用门禁并重新进入队列；不得 `--admin` 或直接推送 main。

## Acceptance Criteria

- AC1 / R1–R3：FDE 与 macOS 相关 owner 的当前签名、封闭集合、优先顺序、失败
  行为和测试断言与实现逐项一致；没有任务证据污染稳定 SPEC。
- AC2 / R4–R5：托管订阅代理沿“账号 -> Provider -> listener -> Agent 配置 ->
  请求 -> overview”只有一组语义 owner；命令 DTO、错误码、目标差异、secret 边界、
  401/刷新与 overview parser 不变量均可由测试直接断言。
- AC3 / R6：必要的 SPEC 拆分/重命名完成，前后端索引及全部相对链接有效；
  `task.py validate` 不报告缺失文件或 required SPEC 截断。
- AC4 / R7：聚焦测试、格式/契约检查和完整 `check:prearchive` 退出码为 0；若只有
  SPEC/任务改动，也不得跳过仓库约定的契约与生命周期门禁。
- AC5 / R8：工作提交、任务归档和会话记录顺序正确，归档后 `check:contracts`
  通过且工作树干净；PR 精确 head 的检查通过，Merge Queue 完成 merge-group 检查，
  最终 `origin/main` 可读回本次 merge commit。

## Out of Scope

- 不新增提示词/MCP 场景、账号提供方、Agent 目标、模型、代理协议、数据库 schema、
  登录方式、公开网关或 per-client key 产品面。
- 不修改上游 MCP 包版本策略，不执行客户云/数据库写入，不使用真实订阅账号发起
  无人值守推理。
- 不重写已归档任务的历史验收记录；只在当前稳定 SPEC、必要实现/回归和本任务
  生命周期材料中收敛结论。
- 不把浏览器夹具、合成 OAuth/upstream 响应、macOS 当前宿主测试或 GitHub CI
  说成 Windows 真机、签名发行、真实授权/额度或所有模型可用性验收。

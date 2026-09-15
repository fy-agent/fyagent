# 复核今日变更的 SPEC 并完成合并

## Goal

复核 `dev/laiyongjie` 在 2026-09-15 新增的首次使用推荐引导及 Grok Build
纠错提交，使实现、测试与 Trellis SPEC 的可执行契约一致；完成本地质量门禁、
任务归档、PR、PR/Merge Queue CI 和最终 `main` 合并读回。

## Background

- 当前分支相对 `origin/dev/laiyongjie` 领先六个提交，产品范围是软件目录正向简介、
  仅新安装可见的可跳过推荐引导，以及补齐 Grok Build 编程推荐。
- 原任务已经各自归档。本任务只评审它们形成的当前行为，不改写历史验收记录。
- 当前实现跨越设备本地设置、Rust 命令与 ACL、Renderer 端口/查询、目录页面、
  浏览器回归和生产分包，因此必须按跨层 code-spec 深度复核。
- 评审发现普通 `save_settings` 仍可能通过旧兼容字段结束引导；推荐理由表也允许
  未来目录身份回退到通用目录简介，二者都削弱了既有窄边界。

## Requirements

- R1：逐项核对今天六个提交的实现、测试、任务记录和相关 SPEC，确认每个稳定
  行为只有一个语义 owner；必要时补充、改写或拆分，避免把一次性执行证据写入
  SPEC。
- R2：首次安装资格必须由原生设备状态 owner 判定。只有数据库、旧配置和
  `settings.json` 均确认不存在时才能初始化 `pending`；存在、损坏、不可读或
  检查失败均不得推断为新安装。
- R3：`firstUseGuideState` 与旧兼容确认字段均为原生维护字段。普通整份设置保存
  必须原样保留最新值，不能打开或结束引导；仅无参数窄命令可以持久化完成状态。
- R4：办公、编程、混合推荐继续与已解析目录取交集并保留目录顺序。当前七个目录
  身份必须拥有显式用途说明；不得用通用目录简介静默兜底新身份。
- R5：保留现有产品边界：推荐不安装、不登录、不改模型、不上传用途；有效目标
  链接优先；目录读取失败不写确认；隐藏/卸载期间不抢焦点或启动扫描。
- R6：补齐与修复相匹配的回归，执行 Trellis 上下文校验、聚焦检查和完整归档前
  门禁；不得用浏览器夹具声称完成真实新安装或跨平台安装器验收。
- R7：完成任务归档和归档后契约检查，仅在最终差异、工作树和 base drift 均已
  复核后推送精确 PR head；通过 PR CI 与 Merge Queue `CI / Required` 后合并，
  读回最终 `main` merge SHA。不得使用 `--admin` 或直接推送 `main`。

## Acceptance Criteria

- AC1 / R1：前后端首次引导 owner SPEC 均包含 Scope、Signatures、Contracts、
  Validation/Error Matrix、Good/Base/Bad、Tests Required、Wrong vs Correct；索引和
  相邻目录/文案契约只保留自己的边界，没有重复第二套状态机。
- AC2 / R2–R3：回归证明现有 `pending` + 普通设置入参中的旧确认值不能结束引导，
  已完成状态也不能被旧快照重新打开；专用命令仍是唯一完成路径。
- AC3 / R4：推荐理由映射对封闭 `AgentCatalogId` 穷尽，当前目录单用途并集仍覆盖
  七个身份，Grok Build 顺序、名称和说明回归继续通过。
- AC4 / R5：现有首次读取失败、显式 target、保存失败/防重入、隐藏完成、最小
  视口、懒加载和正向目录文案测试保持通过。
- AC5 / R6：聚焦测试和完整 `check:prearchive` 退出码为 0，任务 context 校验通过；
  真实未执行项与证据边界记录在任务中。
- AC6 / R7：任务成功归档，归档后 `check:contracts` 通过；PR 精确 head 的检查通过，
  Merge Queue 完成 merge-group 检查并在 `main` 形成可读回的 merge commit。

## Out of Scope

- 不新增软件、用途选项、推荐服务、遥测、安装/登录/模型能力或数据库 schema。
- 不改变三组推荐名单、目录顺序、主路由、设置文件位置或现有包体积上限。
- 不重写两个已归档任务的历史材料，不清理与本次契约无关的旧设置字段或前端类型。
- 不把本机自动化升级为 macOS/Windows 物理全新安装、签名或发行验收结论。

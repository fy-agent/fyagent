# FyAgent v2 QoderWork / TRAE Work P0 实质能力

## Goal

在不猜测厂商私有配置、不扩大 Tauri 权限、不保存用户凭据的前提下，让 FyAgent 对 QoderWork 和 TRAE Work 提供可自动验证的本地 Skills、Qoder Hooks、TRAE 模型连接预检和 MCP 配置预检能力，并让 Agent/Models 目录使用同一套稳定的几何、图标和交互合同。

本任务完成口径是 P0 代码与自动化证据完成。真实应用识别、启动、Skill 被厂商加载、Qoder Hooks 重启后生效等 HIL 项不执行，并保持 `unverified`。

## Background and Confirmed Facts

- 当前基线是干净的 `dev/laiyongjie@c4f7a9c48b86ed670a00f583e5f3bf76e49e7a60`。
- Agent/Models 已共享五个候选及 catalog v2 顺序，但页面仍维护不同的 rail 比例、行高、图标尺寸和模型页 900px 分支。
- QoderWork/TRAE Work 模型页仍包含只存在于组件内存的说明性备注字段；它们不产生权威配置结果。
- 现有 Skills/MCP 分配合同使用六个广义 `AppType` 目标；QoderWork/TRAE Work 不应进入 Provider、proxy、prompt 或 session 语义。
- `SkillService` 已拥有下载、归档、SSOT、copy、冲突与重读能力；WorkBuddy 已拥有 revision、一次性 overwrite token、凭据脱敏和 Windows 路径身份保护模式。
- QoderWork 官方文档公开全局 Skills 目录和 `settings.json#hooks`，并明确 Hooks 修改后需重启。
- 中国版 TRAE Work 官方文档公开全局 Skills 目录、自定义模型表单语义和 MCP 的 stdio/HTTP JSON 形状，但没有公开第三方直接写入模型/MCP 私有存储的合同。

## Product Decisions

- 本任务只交付 P0 自动化范围；P1/P2 延后。
- Agent/Models 保留现有五候选顺序和 Codex 安装器/官方链接行为。
- Skills 目标扩展到八个；MCP direct-sync 目标仍为现有六个。
- Qoder/中国版 TRAE Skills 首版默认 copy，不启用 symlink。
- Qoder MCP 与 TRAE 模型/MCP 首版只做验证、准备和厂商 UI 引导，不写厂商私有配置。
- TRAE endpoint probe 复用 FyAgent 现有全局/系统/直连代理选择，但 SSRF 与 DNS pin 仍为硬门禁；代理无法保持 pin 时 fail closed。
- MCP 只允许复制无敏感值模板；完整 env/header secret 不进入默认剪贴板。
- 不执行任何本机或外部 HIL；不因缺少 HIL 阻止本任务按自动化口径归档。

## Requirements

### R1 — Shared Catalog Geometry

- Agent/Models 必须复用同一个 `CatalogMasterDetail` 组件、rail/list/detail primitives 和尺寸 token。
- rail 使用单一 `clamp(220px, 24vw, 268px)`，gap 为 14px，row 至少 56px，列表图标框 36px，详情图标框 64px，堆叠断点为 760px。
- 页面不得再声明目录 `grid-template-columns`、品牌 ID 图标特例或额外 900px rail 比例分支。
- route/target 切换不得动画 rail、frame 或文字起点；必须支持 reduced motion、稳定 scrollbar gutter、键盘和可见焦点。

### R2 — Typed Brand Metadata

- 每个 Agent 资产必须拥有 typed list/detail optical scale、background 和 corner metadata。
- 列表和详情都通过共享 frame 渲染；图标在已有品牌名称时为装饰图。

### R3 — Catalog v3 and Runtime Separation

- `get_agent_catalog` 升级到 contract v3，声明封闭 capability ID、mode、reason code、evidence ID、variant 和 review date。
- 静态目录必须保持确定性、无网络、无 secret、不读取本机。
- 旧 v2、未知版本、未知 capability、重复 ID 和无效链接必须 fail closed。
- 本机 runtime status 使用独立命令和 `boolean | null`；未知状态不得映射为未安装或 false。

### R4 — Evidence-Bounded Probe and Launch

- renderer 只传 Agent ID 和封闭 destination，不传任意 URL、路径或 executable。
- 没有可信 executable/bundle/signing identity 时，detect/launch 保持 `unverified`，不猜正向候选。
- 失败显示受控 reason code；不得从配置目录、产品名或静态 catalog 推断安装、运行或登录。

### R5 — Skills Domain Separation and Compatibility

- 新增独立 `SkillTargetId`，前六个显式适配现有 `AppType`，QoderWork/TRAE Work 不进入其他广义域。
- `SkillApps` additive 增加默认 false 的 QoderWork/TRAE Work 字段；旧持久化记录无损读取，原六目标值不变。
- 前端分别维护八个 Skill targets 和现有六个 MCP direct-sync targets。

### R6 — Safe QoderWork and TRAE Work Skill Sync

- 目标路径由后端可信 home 与固定相对路径计算，renderer 不提供路径。
- 继续复用 `SkillService` 的 SSOT、归档预算、copy、冲突、hash 与权威重读。
- link/reparse/hardlink、路径逃逸、异常 leaf、TOCTOU 和 hash 漂移必须 fail closed。
- 成功文案只能声称目录同步完成；厂商识别保持 `unverified`。

### R7 — Qoder Hooks Read and Save

- 固定读取 `<home>/.qoderwork/settings.json`，有界到 2 MiB；不存在、空 hooks、非法 JSON 和非法 hooks 类型有明确结果。
- renderer 只得到受支持投影、opaque revision、exists 和恒为 true 的 restartRequired。
- 保存只替换 `hooks` 键并保留未知顶层 JSON；无法无损投影的 hooks 结构阻止结构化保存。
- 保存必须使用 expected revision、锁、必要时 request-digest overwrite token、备份、同目录 temp、flush/sync、原子替换和权威重读。
- validation 不执行 Hook command；保存成功只声称文件已保存并需要重启。

### R8 — TRAE Model Validation and Endpoint Probe

- 模型表单支持封闭 API format、base/complete URL mode、model ID、短生命周期 API Key、no-key/loopback/private consent。
- API Key 只存在于当前组件 state、一次 invoke 和当前请求 header；所有终态、切换与卸载清除。
- endpoint probe 默认 HTTPS、零重定向、3 秒 connect、10 秒 deadline、1 MiB body 上限、禁用压缩展开并支持取消。
- URL、全部 A/AAAA、private/loopback/metadata、DNS rebinding 和代理 tunnel 必须在连接前 fail closed。
- 成功只表示本次预检成功，不表示 TRAE 已保存或完全兼容。

### R9 — MCP Validation Without Execution

- 接受严格 `mcpServers` object；每项恰好是 stdio 或 HTTP transport。
- stdio 只安全解析 executable 是否存在，保持 args 数组，不执行 command、不安装依赖。
- HTTP 只做 URL/地址策略检查，首版不联网。
- env/header secret 只返回存在标记和 reason code，不返回值；只可复制无敏感模板。

### R10 — Minimum Tauri and Secret Boundary

- observe、launch、Qoder write、endpoint probe 使用窄 command/permission；不新增 renderer 通用 fs/shell 权限。
- secret 不得进入 URL、query/hash、storage、React Query、日志、错误、telemetry、snapshot、reason code、revision、token 或默认剪贴板。
- 后端错误使用封闭 code，前端通过本地 i18n 映射，不回显用户输入或外部响应正文。

## Acceptance Criteria

- [x] AC1：四个维护 viewport 下 Agent/Models rail x/width 差不超过 1px；所有列表 frame 为 36×36px，row 高差不超过 1px，760/761px 行为一致。
- [x] AC2：两页共享 catalog primitives 和 tokens；无页面级目录 grid、品牌 ID CSS、额外 rail 断点或几何动画。
- [x] AC3：catalog v3、runtime status 和所有新增 wire 在 Rust/TypeScript/fake-Tauri 中精确一致；v2/未知输入 fail closed。
- [x] AC4：detect/launch 只使用可信 adapter；缺少身份证据时状态与操作保持未验证/禁用。
- [x] AC5：旧 Skills 数据无回归；八个 Skill target 与六个 MCP target 分离；Qoder/TRAE copy、冲突、异常路径和重读测试通过。
- [x] AC6：Qoder Hooks 未知顶层字段保留，revision/token/concurrency/backup/atomic/reread 安全路径和失败路径通过；UI 明确重启要求。
- [x] AC7：TRAE 模型结构校验、代理/SSRF/DNS/deadline/body/cancel 测试通过；任何结果不声称已保存到 TRAE。
- [x] AC8：MCP validator 覆盖 stdio/HTTP union、command resolution、URL、secret 和 oversized input，且证明不启动任何 server。
- [x] AC9：secret sentinel 在日志、错误、DTO、DOM、query、storage、URL、snapshot 和剪贴板模板中为 0。
- [x] AC10：适用 focused checks、Playwright geometry、renderer build、完整 `check:prearchive`、diff/repository hygiene 全部通过。
- [x] AC11：依赖 HIL 的能力保持 `unverified`，未执行 HIL 在任务与最终汇报中明确列出。

## Out of Scope

- 项目级 TRAE Skills、Qoder MCP 完整生成器、证据详情页和版本 allowlist。
- 直接写 Qoder/TRAE 模型或 MCP 私有存储。
- 自动安装/升级 QoderWork 或 TRAE Work、MCP server 试运行、UI 自动化或深链猜测。
- 登录、订阅、组织权限或厂商 token 探测。
- 本机或外部 HIL、推送、PR、发布、签名和安装。


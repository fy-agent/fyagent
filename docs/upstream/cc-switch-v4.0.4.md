# CC Switch v4.0.4 integration ledger

## Status

**400c：工作树冲突清零，Windows GNU all-targets 编译通过；原生运行验收未完成，严格 Clippy 未过。** 2026-10-08。
341 个 DU 已由军师处理；续跑 177 个文件均已在工作树处置并登记。
本轮只读 Git，未标记索引、提交、推送、修改 remote 或创建 PR。
索引状态由军师按交付 TSV 统一处理，不能把未合并索引计数当成工作树进度。

## Immutable source identity

| Item | Identity |
| --- | --- |
| FyAgent baseline | `7097d86a3a98d86cb79d4c0332c8fadb7aa4960d` |
| Branch | `migrate/400-ccswitch-4.0.4` |
| Upstream tag | `v4.0.4` |
| Upstream commit | `a29a4f3868e9c6ffc2212957cfa66a718642e344` |
| Delta | v3.20.4 的后继，276 commits |
| Commit | 无，用户要求仅准备工作树 |

之前的边界与 fix1/fix2/fix3/#207 见 [v3.20.4 账本](./cc-switch-v3.20.4.md)。

## 冲突和取舍

| 功能块 | 取舍 | 实施与边界 |
| --- | --- | --- |
| live 关键字段引擎 | 适配 | `live/**` 接 floor、residue、pending/recovery、保格式字段投影；staged commit 仍经过 FyAgent 备份/undo owner。 |
| Grok / Grok Builder | 适配 | `live/project/grok.rs`、`services/provider/grok_direct.rs` 接既有路径与 TOML 验证；普通切换使用字段级投影，保留用户字段和非目标模型。 |
| Codex / managed auth | 适配 | 保留 SecretRef、登录 owner、Windows 用户态路径和命令边界；普通来源切换不得更改官方 auth、marker 或 stash。 |
| Aggregation 后端 | 带进 | `mode/**`、catalog/Stack/proxy API 已带入；新旧接管状态兼容仍需原生回归。 |
| Aggregation 前端与启动启用 | 需 William 定 | 本轮不增加入口、不用上游自动 attach 替换 FyAgent managed-auth 启动恢复；保留既有 Profile/Quick Setup 行为。事项 1。 |
| 数据库版本 | 适配 | `SCHEMA_VERSION=27`；启动保护比较该常量，v27 可进入初始化，未来版本仍拒绝。上游结构增量为 `enabled_pi`，延期而未引入，不虚增 v28。 |
| 同机数据隔离 | 已处理 | 数据库、live-state、备份均使用 FyAgent 根与文件名；不探测、迁移或清理 CC Switch 数据。第三方客户端文件仍是用户明确选择写入的共同文件。 |
| Pi | 需 William 定 | 继续延期；不注册 AppType、MCP、Prompt、会话、预设和入口。事项 2。 |
| Mcode | 需 William 定 | 继续延期；保留之前未接线源码基线，不启用新接口。事项 3。 |
| 其他新客户端 | 不要（本段无新增） | 对照本次 AppType 变化未发现新的枚举客户端；既有支持清单不扩张。 |
| MCP / Skills / Prompt | 适配 | 保留 #207 状态锁/SSOT 别名防护、九目标与既有返回契约；吸收同目标 MCP 逐条容错等通用修复。 |
| Skills 新错误报告界面 | 需 William 定 | 新 report DTO 会改变既有 API 和界面，本轮保留现状，单列后续入口。事项 4。 |
| 代理 / 会话 | 适配 | 协议、OAuth header、OpenCode V2/分块读取等修复；新 SessionMessage 保留旧界面需要的 content 字段。 |
| 数据库导入 / 备份 | 适配 | 导入前校验、保留本地 session 表、最终锁内复制最新本地状态、精确备份及恢复源保留。 |
| 新界面 / i18n / whats-new | 不要 | 删除纯上游新增组件、对应新 UI 测试和资源；FyAgent renderer 与中文文案保留。 |
| 点星条 / 关于页 / partners | 不要 | 不加入上游产品品牌或推广元数据；预设保留 FyAgent 无商业追踪边界。 |
| 一键更新 | 不要 | 指定 UpdateContext/updater/manifest-rewrite 文件不带入；Tauri 使用 FyAgent 配置，更新另分支负责。 |
| CI / ISSUE_TEMPLATE / CONTRIBUTING | 取我们 | 保留 FyAgent 发布、治理与平台契约。 |
| 依赖 | 取我们 | package/Cargo 清单与锁以 FyAgent 为准；新核心未要求额外依赖，pnpm frozen install 可用。 |

## 指定遗留 CI 问题

- Windows 单实例：400c 使用 Windows cfg 内 builder shadowing 接续所有权，无不可变 binding 二次赋值，也不增加 macOS unused_mut。
- OAuth：恢复账号 header 注入，消除写后未读变量；身份常量统一使用现有 owner。
- 未用项：恢复真实调用或删除已确认无调用的延期 helper；没有大面积 warning allow。
- 数据库导入在补表前检查原始 schema；最终锁内保留本地数据和 receipt、备份与替换。v27/隔离回归已通过 Windows 目标编译，未执行原生测试。
- 旧 CI 的全部 macOS/Windows 失败不能宣称已绿，必须在支持宿主重跑。

## 验证边界

- 前端 typecheck、lint、format 检查通过；领域与既有 IPC/MCP/Skills 相关定向测试已运行，最终数量以本地报告为准。
- 第一轮本机默认目标检查被 helper 平台边界阻塞（历史证据保留）；400c 改用已安装的 Windows GNU 目标，`cargo check --target x86_64-pc-windows-gnu --all-targets -j 6` 退出 0。严格 Clippy 的剩余 dead-code / 测试模块位置告警见本轮报告；没有通过桩或放宽 cfg 绕过。
- v27 证据包含来源于源码的 SQLite DDL 执行、未来版本保护和独立数据库字节不变检查；Rust `Database::init` 升级测试尚未运行。证据为 **code_audit + SQLite DDL execution**。
- 正式 `supported-platform-check` 因未合并索引多 stage 拒绝重复路径；工作树审计、结构指纹更新另存证据。军师标记后必须重跑正式脚本。
- 保留原生金标回归，适配产品身份与 FyAgent auth/深链契约；不能声称已取得运行时字节对比结果。

## 本地交付

仓库外文件位于工作树父目录：`report-400.md`、追加历史 `p400-resolved.tsv`、
最终去重清单 `p400-resolved-final.tsv`、`p400-evidence/`。
报告保存完整命令、真实退出码、失败摘要、剩余风险和逐模块审阅记录。
`merge-400.patch` 表示 HEAD 到工作树的内容；索引快照另存，不能替代最终补丁。

## Superseded evidence

第一版“518 项未处理、必须恢复索引写权限”已被用户续跑指令撤销。
从本轮开始只修改工作树，所有索引标记由军师完成；旧失败日志仅作为历史保留。

## 400c 收尾增量

- Gemini #358 统一使用 `live/**` / `gemini_direct`：删除旧 provider writer、整份 env 序列化和 OAuth/PackyCode settings 直写；代理兼容入口及已证明归属的恢复也走引擎。损坏 settings 拒绝并保留 env/指针的回归已加入。
- 前端会话 reader 从 blocks 生成 content，允许原生扩展字段；包导入/恢复的严格边界不变。当前前端没有调用 streaming reader，无新增 UI/i18n。
- fix4 69 项失败测试均有现存名称对应（62 原名、7 已记录改名）；名称覆盖和交叉编译不是运行通过。
- 补回 OpenClaw 所需共享 JSONL block parser；没有启用 Pi 客户端。修复失效 IPC 注册、OpenCode V2 格式传递和误合的测试夹具。
- [400c 报告](/workspace/fy-maint-1007/migration/report-400c.md)、[逐项审计](/workspace/fy-maint-1007/migration/p400c-evidence/audit-final.json)、[改动清单](/workspace/fy-maint-1007/migration/p400c-changed.txt) 为当前收尾证据，前两轮报告仅作历史。

# 本机 provider 差异来源只读核对

结论：现有应用日志直接记录了与主控差异报告逐项一致的 5 次导入、2 次更新。它们发生在 2026-09-19 18:14:36 的启动过程中，来源标为 live config；因此该 provider 差异可由启动自动导入解释，有运行日志支撑，不能归因于 v21→v22 schema 迁移本身。精确启动二进制路径、PID、操作者及当时 FYAGENT_TEST_HOME 值，现有日志不能确认，标记 **unknown**。

## 直接证据

主控材料 `control-evidence/live-data-preservation-before-install.json` 记录 providers 13→18；新增四个 OpenCode 和一个 OpenClaw 身份，zai/openclaw、vibekey/openclaw 仅 settings_config 改变，另六张业务表相同。本检查沿用该差异结果，没有重新读取受限备份或业务 DB 的配置内容。

真实应用日志 `~/.fyagent/logs/fyagent.log`：

| 行 | 本地时间 | 脱敏后的事件 |
|---|---|---|
| 35749 | 2026-09-19 18:14:35 | FyAgent v0.4.5 started |
| 35751 | 18:14:35 | Creating pre-migration database backup (v21 → v22) |
| 35755–35758 | 18:14:36 | Imported OpenCode providers google、openai、poe、zai-coding-plan from live config |
| 35759 | 18:14:36 | Synced 4 OpenCode providers from live config |
| 35760 | 18:14:36 | Imported OpenClaw provider bailian from live config |
| 35761–35762 | 18:14:36 | Updated OpenClaw providers zai、vibekey from live config |
| 35763 | 18:14:36 | Synced 3 OpenClaw providers from live config |

日志中的身份、导入/更新类型和数量与差异报告完全吻合；并非只有“可能自动导入”的源码推断。日志不包含具体设置值，也不足以反推值是否曾被外部编辑或由哪个程序编辑。

## 代码对应关系

- `src-tauri/src/lib.rs:1463` 附近为每次启动的 additive provider 导入编排，1472 调 OpenCode、1479 调 OpenClaw；不要求 providers 表为空，且在 UI 操作之前执行。
- `src-tauri/src/services/provider/live.rs:1905` 的 OpenCode 导入：读 live provider 对象，缺失 ID 新增；已有 ID 若 settings/name 不同则保存。成功后才打印 Imported/Updated 日志。
- 同文件 `:1989` 的 OpenClaw 导入：已有 ID 比较 settings_config，只替换 settings_config 并保存，保留其他 provider 字段；缺失 ID 新增。正好对应两条旧记录只变 settings_config 的形状。
- `src-tauri/src/opencode_config.rs:179` 从配置的 provider 对象读取；`src-tauri/src/openclaw_config.rs:658` 从 models.providers 读取。这两个导入函数将值存入数据库，本身不调用写回 live 配置方法。
- `src-tauri/src/database/schema.rs:557` 的 21→22 分支只调用项目表/资源代际初始化及 verification 表初始化，然后推进 schema version；所核对迁移函数不新增业务 provider 或替换 settings_config。资源代际 seed 写的是本地计数表，非业务 provider。
- `src-tauri/src/config.rs:53` 在 macOS 优先采用非空 FYAGENT_TEST_HOME，否则采用真实用户 home；配置目录也可有既有覆盖设置。故“短暂无测试 HOME 启动”与日志一致，但本检查没有该进程的环境/二进制指纹，不能据此确认是某一个候选进程。

## 边界

本检查仅读取限定源码、主控差异 JSON 和日志中的白名单事件；未输出设置、凭据或完整 provider JSON，未修改 live DB、配置或应用状态，未启动应用，未跑迁移或导入。未将“来源可解释”升级为“数据完全未变”或“全局配置未被写入”的结论。需要恢复或改变启动导入策略时应由主控另行决策，本次不自动处理。

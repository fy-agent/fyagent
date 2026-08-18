# Prompt / Memory 产品模型与现有后端差距

## 1. 评估结论

新前端**不能直接把现有 Prompt/Memory API 接上就上线**。现有后端可以作为少量读取和文件 I/O 基础，但它表达的是旧模型：

- Prompt：按 `app_type` 存储，一次只启用一条，正文覆盖整个目标文件。
- Memory：只覆盖默认 OpenClaw workspace 的固定文件/daily 和 Hermes 两个整段 Markdown。

新页面需要的本机工具、实例、workspace、资源能力、多规则组合、会话索引、来源追溯和逐目标状态目前均没有统一合同。本轮前端因此保持 `prototype`，不调用旧写入接口伪装新能力。

## 2. 代码依据

### Prompt

- `src/lib/api/prompts.ts`：前端 `Prompt` 类型与 6 个 invoke 方法。
- `src-tauri/src/prompt.rs`：Rust DTO 只有基础字段。
- `src-tauri/src/database/schema.rs`：主键是 `(id, app_type)`。
- `src-tauri/src/services/prompt.rs`：互斥启用、整文件写入、live 文件回填/备份。
- `src-tauri/src/prompt_files.rs`：按 `AppType` 解析单一目标文件。

### Memory

- `src-tauri/src/commands/workspace.rs`：默认 OpenClaw workspace 白名单文件与 `memory/YYYY-MM-DD.md`。
- `src-tauri/src/hermes_config.rs`：Hermes `MEMORY.md / USER.md` 整段读写、字符预算和 enabled flag。
- `src-tauri/src/commands/hermes.rs`：Hermes memory Tauri commands。

本机其他真实来源见 `local-agent-inventory.md`，当前仓库没有对应统一后端入口。

## 3. 本机扫描：目前完全缺失的一层

### 页面需要

```ts
type LocalAgent = {
  toolId: string;
  version: string | null;
  detected: boolean;
  instances: Array<{
    instanceId: string;
    name: string;
    workspaceId: string | null;
    workspacePath: string | null;
  }>;
};

type LocalResource = {
  id: string;
  toolId: string;
  instanceIds: string[];
  workspaceId: string | null;
  semanticType: string;
  storageKind: "markdown" | "json" | "jsonl" | "sqlite";
  path: string;
  exists: boolean;
  readable: boolean;
  writable: boolean;
  searchable: boolean;
  itemCount: number | null;
  updatedAt: number | null;
  owner: "prompts" | "memory";
};
```

### 现有后端能否取得

| 字段 | 现状 | 直接联调影响 |
| --- | --- | --- |
| `toolId` / 固定 app 枚举 | 部分有 | 只能表达已知应用，不能返回“检测到了什么” |
| 工具版本、`detected` | 无统一接口 | “重新扫描本机”无法真实执行 |
| `instanceId / instanceName` | 无 | 看不到 OpenClaw `main / utility / group_liaison` |
| `workspaceId / workspacePath` | 无统一模型 | 无法区分两个 OpenClaw workspace |
| resource ID 与 semantic type | 无 | 前端无法稳定区分 instruction、user profile、memory、session |
| `storageKind` | 无统一字段 | 无法决定 Markdown 编辑或 SQLite 只读/搜索 |
| exists/readable/writable/searchable | 部分函数可推断，未返回 | UI 只能硬编码能力，错误状态不可靠 |
| `itemCount / updatedAt` | OpenClaw daily 部分有 | 其他来源的数量和更新时间取不到 |
| canonical path / shared instances | 无 | `main + utility` 会重复写同一路径 |

### 需要工作

1. 增加只读扫描层，逐工具发现版本、实例、workspace 和资源。
2. 统一路径解析与 canonical path 去重，结果保留所有覆盖实例。
3. adapter 返回 capability，不由前端根据扩展名猜读写能力。
4. 扫描只返回元数据；正文、会话和搜索结果按 resource ID 延迟读取。
5. 为不存在、权限不足、格式不兼容、被占用等状态提供稳定 error code。

## 4. Prompt：哪些字段能取到

现有 `Prompt` DTO 可直接返回：

| 字段 | 可取 | 备注 |
| --- | --- | --- |
| `id` | 是 | 只在某个 `app_type` 内有意义 |
| `name` | 是 | 可复用 |
| `description` | 是，可空 | 可复用 |
| `content` | 是 | 是整条正文，不是 compose 结果 |
| `enabled` | 是 | 当前含义是同 app 互斥启用 |
| `createdAt / updatedAt` | 是，可空 | 可复用 |
| 当前目标文件正文 | 是 | `get_current_prompt_file_content(app)` 返回字符串或 `null` |

现有命令还提供按 app 获取、upsert、delete、enable、从文件导入和读 live 文件。

## 5. Prompt：页面取不到的字段与问题

| 页面字段/能力 | 当前是否可取 | 直接使用会发生什么 |
| --- | --- | --- |
| 全局规则身份 | 否；主键是 `(id, app_type)` | 同一规则在多个工具中变成多份副本，内容会漂移 |
| `category` / `origin` / 内置版本 | 否 | 无法区分本机提炼、官方模板、自定义和安全升级 |
| `targetResourceIds[]` | 否 | 一条规则不能一次分配多个真实文件 |
| 目标实例/workspace/path/exists | 只能按 app 内部解析 path，未返回 DTO | UI 无法显示或确认实际注入位置 |
| 每目标启用关系 | 否 | 不能表达同一 Agent 同时启用多条规则 |
| `sortOrder` | 否 | compose 顺序不稳定 |
| managed block marker/hash | 否 | 无法只更新 FyAgent 区块 |
| preview/diff | 否 | 用户看不到最终写入内容 |
| `status / lastSyncedAt / errorCode` | 否 | 不能显示逐目标结果和部分失败 |
| external change/conflict | 否 | 外部改动可能被静默覆盖 |
| path dedupe / covered instances | 否 | 共享 workspace 可能被重复写入 |

### 现有写入为何不能复用

- `enable_prompt` 先把同 app 所有 Prompt 设为 disabled，再只启用目标项。
- `upsert_prompt(enabled=true)` 和 `enable_prompt` 都把 `prompt.content` 写成目标文件完整内容。
- 当所有 Prompt disabled 时，`upsert_prompt` 会清空目标文件。
- `enable_prompt` 会先把 live 文件回填到旧启用项或创建备份；这不是受管区块合并。

因此，前端循环调用 `enable_prompt` 或先拼接字符串再 upsert，都会破坏新页面的多规则语义，并存在覆盖用户文件的风险。

## 6. Prompt：需要的后端工作

1. 数据迁移为 `prompt_rules` + `prompt_rule_targets`（或等价 many-to-many）。
2. 增加 `category / origin / builtinVersion / customizedAt`。
3. 目标引用本机扫描得到的 resource ID，不再只传 `app`。
4. 增加 per-target `enabled / sortOrder / status / lastSyncedAt / syncedHash / errorCode`。
5. 增加 compose preview，返回最终内容、来源规则、目标路径和 diff。
6. 用受管区块保护外部内容；写前备份/hash，原子写，写后回读。
7. canonical path 去重；一个资源结果同时返回覆盖实例。
8. 旧数据迁移时保留原 Prompt 和 live 文件快照，不自动覆盖用户定制。

## 7. Memory：现有后端能取到什么

### OpenClaw 默认 workspace

| 能力 | 可取字段/结果 |
| --- | --- |
| 列 daily 文件 | `filename / date / sizeBytes / modifiedAt / preview` |
| 读/写/delete daily | 文件名 + 整段 content；文件名必须为 `YYYY-MM-DD.md` |
| 搜索 daily | `filename / date / sizeBytes / modifiedAt / snippet / matchCount` |
| 读/写核心 workspace 文件 | 白名单文件名 + 整段 content |
| 打开目录 | `workspace` 或 `memory` |

限制：所有路径都固定在 `get_openclaw_dir()/workspace`，没有 instance/workspace 参数，所以无法读取 `workspace-group_liaison`，也无法返回 `main + utility` 的共享关系。

### Hermes

| 能力 | 可取字段/结果 |
| --- | --- |
| 读/写长期记忆 | `MemoryKind::Memory` 的整段 Markdown |
| 读/写用户资料 | `MemoryKind::User` 的整段 Markdown |
| 限制 | memory/user 字符预算、两个 enabled flag |

限制：当前命令不列举 session、message、FTS/trigram 结果，也不向 Memory UI 返回文件 lock/外部版本状态。整段写入接口不等于跨 Agent 同步 adapter。

## 8. Memory：页面取不到的字段与来源

### 所有来源共同缺失

| 页面字段/能力 | 当前状态 | 影响 |
| --- | --- | --- |
| resource ID / semantic type / owner | 无统一字段 | Prompt 与 Memory 可能争抢同一文件 |
| source tool/instance/workspace | 无统一字段 | 提炼后无法证明来自哪里 |
| storage kind/capability | 无统一字段 | 前端只能硬编码只读、可写、可搜索 |
| 长期记忆条目 ID 与类型 | 只有整段文件 | 不能管理单条偏好、事实、决定、经验 |
| `sourceEntryId / capturedAt / contentHash` | 无 | 无法追溯、去重或识别重复提炼 |
| 目标关系 | 无 | 不能选择同步对象 |
| per-target status/version/error | 无 | 不能显示 partial、冲突或落后目标 |
| preview/verify | 无 | 保存与真实同步无法区分 |
| 跨工具搜索/分页 | 无 | 大量会话不能安全加载和筛选 |

### 当前完全没有 adapter 的本机来源

- Codex：`session_index.jsonl`、rollout、archive、`memories_1.sqlite`。
- Claude Code：`~/.claude/memory/*.md`、history、项目 JSONL、transcripts。
- Gemini CLI：项目 chat JSONL。
- OpenCode：`MEMORY.md`、日期文件、`opencode.db` sessions/messages/parts。
- OpenClaw：三个实例的 sessions/trajectory/checkpoint，以及第二 workspace。
- Hermes：JSON sessions、`state.db` sessions/messages/FTS/trigram。

## 9. Memory：需要的后端工作

1. 建立与扫描资源绑定的 adapter registry，声明 `list/read/search/write/verify` 能力。
2. 为 Markdown、JSON/JSONL、SQLite 分别实现安全读取；会话使用分页和摘要，不一次返回全文。
3. 建立 FyAgent 长期记忆条目，保存语义类型、来源资源/条目、时间、hash 和版本关系。
4. `promote_to_long_term_memory` 只创建草稿，不修改原 daily/session。
5. 建立 `memory_targets` 和逐目标状态；目标仅来自 writable 且语义兼容的资源。
6. 写入前返回 preview、字符预算、lock、外部版本和冲突；写后 verify。
7. 对 Hermes 的字符预算、OpenClaw workspace 分离、共享实例路径去重做专门映射。
8. 搜索优先使用现有索引（Hermes FTS、SQLite 索引等），无索引来源再做受限扫描。

能力判定不能只看文件是否存在：本机 OpenCode 的 `MEMORY.md` 没有被全局指导文件引用，当前只能作为维护来源；Claude 的本机 `CLAUDE.md` 则明确引用其 memory 目录。扫描/adapter 必须返回“原生、规则桥接、普通文件、只读索引”等接入模式，前端据此决定是否列为同步目标。

## 10. 前端对接工作

后端具备新合同后，前端仍需完成：

1. 用 V2 page data source 替换 `prototype.ts`，不让组件直接 invoke Tauri。
2. 增加扫描 loading/empty/partial/error 和 capability-based controls。
3. 路径只用于展示，动作传 resource ID；绝对路径按 UI 需要脱敏/缩写。
4. Prompt 增加组合预览、diff、排序、冲突解决和逐目标重试。
5. Memory 增加分页、来源过滤、搜索结果定位、提炼选择范围和来源跳转。
6. 同步结果按目标渲染，区分 local save、queued、synced、partial、conflict、failed。
7. 新合同 schema 解码、缓存失效和失败路径测试。

## 11. 推荐联调顺序

1. **资源扫描只读**：先让页面显示真实工具、实例、workspace、路径、能力、数量。
2. **Prompt 新存储与 compose preview**：不写文件，先验证多规则/多目标/路径去重。
3. **Prompt 受管区块写入**：从临时副本到 Codex/Claude，再扩到 OpenClaw/Hermes。
4. **Memory Markdown 只读**：Claude/OpenCode/OpenClaw/Hermes。
5. **Memory 会话 adapters**：JSONL/SQLite 搜索、分页和读取。
6. **长期记忆提炼**：建立来源和版本，不做自动广播。
7. **Memory 逐目标同步**：先 USER/MEMORY 语义明确目标，再扩展能力。

每一阶段都先固定 DTO 和错误状态，再接 UI；不要让页面根据某台机器的目录结构继续长出隐式分支。

## 12. 本轮前端边界

- 已实现新信息架构、真实结构展示和完整本地交互闭环。
- 所有“扫描、保存、同步”反馈都明确是前端/本机扫描预览。
- 未调用上述旧写入接口，未修改 `src-tauri`，未触碰真实 Agent 文件。

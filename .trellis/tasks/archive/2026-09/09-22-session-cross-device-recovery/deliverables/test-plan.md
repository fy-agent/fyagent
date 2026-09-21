> 历史准备材料（superseded）：最终实现合同见 `.trellis/spec/backend/session-migration.md`，当前能力与验收见 `implementation/delivery-readiness.md`。下文的 src/v2、Codex inject-only 及早期 provider 缺口判断不作为当前产品事实。

# Session 跨设备恢复测试方案

- 任务：`.trellis/tasks/09-22-session-cross-device-recovery`
- 角色：Cursor 测试架构师
- 执行模型：GPT-5.6 Sol
- 状态：测试设计完成；本交付未运行生产实现测试
- 结构化用例：`deliverables/test-cases.json`

## 1. 测试目标与不可降级判定

测试目标是证明：迁移包只保留每轮用户原文与 AI 最终答复原文，并能在目标设备的对应软件中重建原生会话、重启后读回、继续下一轮。以下规则是发布门槛：

1. 查看、复制、包解析成功、原生写入、原生读回、打开、重启读回、下一轮请求上下文、真实模型续聊、跨 OS 双向是不同验收阶段，证据不得互相替代。
2. 只按 `assistant` 角色过滤不合格。工具调用、工具结果、进度播报、思考、附件、工作文件和隐藏原生日志不得进入迁移包。
3. 用户消息和最终答复正文按原文保留，不摘要、不 Unicode 归一化、不改写正文中的路径或代码。
4. 任意位置的歧义 assistant final 都必须以 `finalAnswerIndeterminate` 阻止整个会话导出。真正没有 assistant 的用户输入不是歧义：必须按原始有序角色序列保留，并明确标为未完成，不生成空 assistant、占位答复或模型补写。连续 user、用户中断后再问都不是天然坏数据，不能为凑 `user → assistant` turn 而合并、丢弃或改写。
5. 导入不得覆盖既有原生会话。目标原生 ID 可以变化，但稳定迁移身份、目标映射和结果阶段必须可追溯。
6. 认证、账号、模型凭据和源设备绝对路径不进入包；目标目录、登录和模型来自目标设备。
7. Session 一级入口替换不允许删除、改名或改写现有 `MEMORY.md`、`USER.md`、每日记忆文件。
8. 七个 provider 都必须有能力检测结果。验证顺序先 Codex/OpenCode，不得把其他 provider 永久隐藏或误标为不可能。

## 2. 当前基线与证据结论

### 2.1 现有代码只能作为读取基础

- `SessionMeta` 只有 provider/session/title/目录/时间/source path/resume command，`SessionMessage` 只有 role/content/ts；没有 turn ID、final 标志、完成状态或可移植身份（`src-tauri/src/session_manager/mod.rs:9-37`；`src/types.ts:457-473`）。
- 七 provider 扫描与读取在 `src-tauri/src/session_manager/mod.rs:58-115` 聚合，但没有会话包导入导出命令。
- Codex 读取把 `function_call` 映射成 assistant、把 `function_call_output` 映射成 tool（`src-tauri/src/session_manager/providers/codex.rs:205-269`）。
- 通用文本提取会展开 `tool_use`、`tool_result`、`input_text`、`output_text`（`src-tauri/src/session_manager/providers/utils.rs:67-127`），所以现有展示 DTO 不能直接作为 final-only 导出源。
- OpenCode 文件与 SQLite 读取会把 tool part 变成 `[Tool: ...]`（`src-tauri/src/session_manager/providers/opencode.rs:157-240,243-329,556-570`）。
- Claude、Gemini、OpenClaw、Grok、Hermes 的当前读取也只是格式级角色抽取，不是 final-only 判定（分别见 `claude.rs:33-86`、`gemini.rs:58-113`、`openclaw.rs:77-123`、`grokbuild.rs:54-90`、`hermes.rs:187-229,431-485`）。
- 现有终端启动器在非 macOS 明确失败（`src-tauri/src/session_manager/terminal/mod.rs:18-47`），不能作为 Windows 恢复通过的证据。
- 现有前端 API 仅列出、读消息、删除和启动终端（`src/lib/api/sessions.ts:13-54`），没有迁移状态接口。

### 2.2 已有隔离证据只证明局部能力

| Provider | 当前证据 | 可以声明 | 仍不得声明 |
|---|---|---|---|
| Codex 0.154.0 | `thread/inject_items` 返回 `{}`；合成消息以 `phase=final_answer` 落入隔离 JSONL并跨 App Server 重启存在；向本机模拟端点发起下一轮时，请求按正确角色、顺序和 phase 带上四条历史；但 `thread/items/list`、`thread/turns/list` 返回空数组 | 低层注入请求被接受、磁盘持久化、重启 resume、下一轮请求上下文构造已在隔离探针中观察到 | 原生历史列表/UI 可见、真实模型回复、Mac/Windows 双向已通过；模拟端点不是模型续聊 |
| OpenCode 1.18.30 | 合成 final-only 原生 JSON 导入/导出成功，4 条消息顺序与正文保留 | 该版本、该隔离存储上的包到原生格式局部往返通过 | 模型续聊、重启后的交互闭环、Mac/Windows 双向、其他版本已通过 |
| Hermes v0.20.5 + 本地补丁 | 合成 Codex 格式经安装版 importer 写入 SessionDB、读回并离线追加用户消息 | 局部原生写入/读回可行 | importer 自身 final-only（实证为否）、真实模型续聊、跨 OS 已通过 |
| Gemini 0.59.0 | help 有 `--session-file` | 能检测到候选入口 | 精简包可导入、跨项目/跨 OS 可恢复 |
| Claude Code 2.1.220 | help 有 resume/continue/fork | 已有原生 resume 能力 | 任意精简问答可导入 |
| Grok Build 1.0.34 | 有 resume/export；本机 help 无 import | 可检测安装版差异 | 网页所述 import 在本机存在、Markdown 可回灌 |
| OpenClaw 2026.7.1-2 | 可列出/读取既有会话；无已核实通用 import | 可读取候选历史 | 精简历史可原生写入 |

证据来源：`research/session-recovery-20260921/local-cli-evidence.json`、`codex-protocol-excerpt.json`、`opencode-synthetic-result.json`、`hermes-synthetic-result.json`、本任务 `evidence/codex-probe/`。另有 `evidence/prototype-contract-test-result.json` 的 6 条隔离 DOM 运行时检查为 passed，只说明原型合同，不是本目录 35 条旧生产用例或本次新增用例通过。本测试交付没有追加运行；Codex request capture 使用模拟端点且 `model_called=false`。

## 3. 被测边界与阶段状态机

建议测试最小充分实现：直接 JSON 迁移包 + **现有 FyAgent SQLite 中一张设备本地 receipt 小表**；不要求 sidecar、通用任务引擎或第二套会话数据库。JSON 导出文件继续使用 `atomic_write`，receipt 则测试 SQL `INSERT` / `UPDATE` / 事务与唯一约束，不测试文件 rename。

### 3.1 导出边界

`provider native source -> provider extractor -> final-only validator -> canonical package serializer -> atomic file write`

导出成功必须同时满足：

- 可迁移正文是原始有序消息序列。user 原文全部保留；assistant 只有版本限定且 fixture 实证为 final 的正文才能保留。
- 缺 assistant 与 `assistant.text == ""` 是两种不同状态；连续 user 不得被折叠。
- 所有正文与 fixture 的预期字符串逐码点相等。
- 包结构通过严格 schema；包内递归扫描没有禁用键和禁用事件。
- 临时文件原子替换成功；失败时目标路径不存在或保持旧文件字节不变。

### 3.2 导入与恢复边界

`strict parse -> schema/size/content validation -> capability probe -> target cwd/model selection -> duplicate/snapshot decision -> receipt slot INSERT -> native write -> receipt UPDATE -> native read-back -> open -> restart read-back -> request-context verification -> real continuation verification`

规范状态：

1. `packageVerified`
2. `nativeWritePending`
3. `nativeWritten`
4. `nativeReadbackVerified`
5. `targetOpened`
6. `restartReadbackVerified`
7. `nextTurnRequestVerified`
8. `nextTurnReplyVerified`

异常分支为 `needsReconciliation`、`ambiguous`、`failed`。

`userAttestation` 是独立字段，不推进上述系统验证阶段。用户确认、改目录或点击按钮都不能解锁 provider capability。任何 UI 或 receipt 不得跳级。

原生操作退出非 0、超时、断连，仍可能已经产生副作用；0 个对账命中也不能证明没有写。此时 receipt 进入 `needsReconciliation`，后续默认重放不得再次调用 native writer。只有 provider 的权威、强相关证据证明该 attempt 无副作用，才可把**同一 request**标成安全重试；已知 target native ID 优先按 ID 对账，没有 ID 时必须排除 developer/环境注入后做精确内容比较。0 个或多项候选都保持待对账，不能盲写。

## 4. 合成 fixture 设计

所有自动化使用隔离 HOME/XDG/APPDATA、临时工作区和假 provider 进程；禁止读取真实用户会话库。

### F-GOLDEN-3T：三轮完整 final-only 基准

- Turn 1 user：`合成档案：纸鹤的编号是 KITE-728。\n保持大小写。`
- Turn 1 final：`已记录纸鹤编号 KITE-728。`
- Turn 2 user：含中文、emoji `🪁`、CRLF、Markdown、用户主动粘贴的 `/Users/alice/demo`、`D:\work\demo` 和代码块。
- Turn 2 中间事件：progress、reasoning、tool call 参数、tool output、附件描述、文件快照，各自带唯一泄漏哨兵。
- Turn 2 final：含代码块、同样的两种路径文本及字符串 `FINAL-T2-KEEP`。
- Turn 3 user：`约定颜色为琥珀色，校验词是 AMBER-314。`
- Turn 3 final：`约定颜色：琥珀色；校验词：AMBER-314。`

正文哨兵必须保留；中间事件哨兵 `LEAK_TOOL_ARG_91`、`LEAK_TOOL_OUT_92`、`LEAK_PROGRESS_93`、`LEAK_REASONING_94`、`LEAK_ATTACHMENT_95`、`LEAK_FILE_96` 在包的 UTF-8 字节中必须全部不存在。

### F-INDETERMINATE

一个 user 后有两条 assistant 可见文本，来源格式没有 completion/final 证据，并夹有 progress。预期 `finalAnswerIndeterminate`，不猜测最后一条。

### F-INCOMPLETE

有 user、tool/progress，但没有 assistant final。预期保留该 user 的原始位置并标记未完成；不生成空 assistant、占位符或 AI 补写。该 fixture 还覆盖未完成 user 位于中间、后续用户再次提问的情形。

### F-ORDERED-USERS

有序序列为 `user(A) → user(B) → assistant(final B) → user(C, unfinished)`。A 表示中断后没有答复，B 是重新提问，C 是尾部未完成输入。预期四条消息按原顺序表示，不把 A+B 合并，不把 final B 配给 A。

### F-IDENTITY-COLLISIONS

- 两个不同来源会话具有完全相同的有序正文。
- 一个来源没有 `sourceSessionId`。
- 一份序列含 `assistant(text="")`，另一份在同一位置缺 assistant。
- 目标 native ID 变化后再导出，包中没有原 receipt 血统映射。

### F-MALICIOUS-PACKAGE

基于合法包分别变异：重复 JSON key、未知字段 `toolCalls`/`reasoning`/`sourcePath`/`token`、路径穿越、绝对目标路径、命令字符串、NUL、超深嵌套、超大正文、错误 digest、错误 schema/version、额外尾随数据。

### F-REVISION

- R1：F-GOLDEN-3T 的前 2 个完整 turn。
- R2：R1 为严格前缀，再增加第 3 个完整响应片段。
- DIVERGED：目标原生会话在 R1 后已有本地第 3 轮，不是 R2 的第 3 轮。

### F-MEMORY-SENTINELS

隔离目录内创建 `MEMORY.md`、`USER.md`、`memory/2026-09-22.md`，记录路径、内容 SHA-256、权限和 mtime；每个导出/导入/重试用例后逐项比较。

## 5. 精确 oracle

### 5.1 原文保真

- 比较 JSON 解码后的字符串码点序列，不做 trim、换行转换、NFC/NFD 归一化或 Markdown 重渲染后比较。
- 顺序 oracle 是消息数组中的 `(message_index, role, completion_state, text)`；不要求交替角色。若实现保留 turn 视图，它只能是派生显示，不能改写原始序列。
- 用户正文里的路径和代码保留；只有包的结构性 cwd 元数据必须被移除或表示为不可信的源提示，绝不能自动成为目标路径。

### 5.2 final-only 与隐私

同时执行两类 oracle：

1. 白名单：包中可迁移消息只出现 fixture 声明的 user/final 字符串。
2. 黑名单：递归 key 扫描和原始字节扫描均不得发现 tool、reasoning、progress、attachment、file snapshot、credential、source absolute cwd 等禁用元数据字段或合成哨兵；用户/最终答复正文中的路径和代码原样保留，不能对正文按关键词删改。

仅黑名单不够，因为未知字段可以夹带内容；仅白名单也不够，因为 envelope 可能隐藏原生日志。

### 5.3 身份、幂等、新快照和另存

- 相同包 + 相同目标 + 默认导入：返回同一目标 native ID，`outcome=already_imported`，原生消息数和文件/DB 行数不变。
- `contentDigest` 只由有序 `(kind byte, UTF-8 byte length, text bytes)` 计算；不含 source/target ID。缺 assistant 与空 assistant 必须得到不同 digest。
- `OriginIdentity` 与 `contentDigest` 分开。有真实 source ID 的同内容不同来源不得因 digest 相同自动合并。没有原生 source ID 时保持 `sessionId=None`，为来源实例保存独立随机 `originId`；同一包复制保留标识，不同未知来源同正文不能折叠，不能凭正文声称同源血统。
- target mapping 只来自当前 target store 的有效 receipt；目标 native ID 变化后不能把该 ID 填回来源公式重算 identity。再导出若丢失来源映射，不得臆造原血统。
- R2 是 R1 的同源较新快照也**不原地追加、不自动合并**；默认停止并提示显式 `SaveAsNewCopy`。目标已分叉同样处理。
- 每次显式 `SaveAsNewCopy` 有新的 `requestId` / `attemptId` 和新 native ID；同一 request 重放幂等，新 request 才创建下一副本。旧副本的 mapping 永不被覆盖。
- 默认导入的唯一 `idempotencySlot` 由 `snapshotId + targetProviderId + targetStoreId + currentDeviceBinding`（长度前缀哈希；snapshotId 由 originId 与 contentDigest 计算） 约束；旧版单一 migration key 不得作为多副本主键。
- 禁止覆盖模式进入首版自动化通过路径；即使 UI 有占位，也不能作为默认或推荐动作。

### 5.4 错误码与边界的唯一命名

测试合同统一使用 serde 风格 camelCase 机器码：

| 行为 | 机器码 |
|---|---|
| final 歧义 | `finalAnswerIndeterminate` |
| 包 malformed / 重复 key | `packageMalformed` |
| v1 未知字段 | `packageUnknownField` |
| schema 不支持 | `packageSchemaUnsupported` |
| 包 / 消息 / 消息数 / session 数量或文本超限 | `packageTooLarge` / `messageTooLarge` / `tooManyMessages` / `tooManySessions` / `sessionTooLarge` |
| JSON 过深 | `jsonTooDeep` |
| 身份字段非法 | `identityFieldInvalid` |
| 目标目录不存在 | `targetDirectoryNotFound` |
| provider 未安装 / 版本不支持 / 能力探测失败 / store 无法识别 | `providerNotInstalled` / `providerVersionUnsupported` / `capabilityProbeFailed` / `targetStoreUnidentified` |
| native 非 0、协议失败、读回不符 | `nativeImportFailed` / `nativeProtocolFailed` / `nativeReadbackMismatch` |
| 需要对账 / 多候选 | `reconciliationRequired` / `ambiguousNativeMatch` |
| 同源新快照或目标分叉需显式另存 | `sourceSnapshotConflict` |

边界固定为：源单会话最多 256 MiB；单消息 4 MiB；单 session 最多 4000 条有序消息且正文总计 32 MiB；单包最多 200 个 session 且文件最多 128 MiB；JSON 嵌套深度最多 64。全部按 UTF-8 字节计数，超限失败且不截断。

### 5.5 坏包与未知字段

v1 采用严格闭集 schema。envelope、session、message 任一级未知字段都返回 `packageUnknownField`；未知 schema/version 返回 `packageSchemaUnsupported`。任何校验错误都发生在 capability probe/native write 之前，provider 隔离存储与 receipt 均无变化。

### 5.6 目录与跨 OS

- 包内 macOS/Windows 源路径不得直接用于目标写入。
- 目标目录不存在时返回 `targetDirectoryNotFound`，不自动创建、不降级到当前目录、不写原生会话。
- 用户选择存在的目标目录后，provider 原生元数据只能包含目标本机路径。
- 正文中用户主动粘贴的旧路径必须原文保留，这与结构性 cwd 映射分开断言。

### 5.7 下一轮请求上下文与真实续聊

续聊测试发送：`只回答纸鹤编号、约定颜色和校验词，用“编号|颜色|校验词”格式；不要读取文件或调用工具。`

请求捕获若证明模型请求中按正确角色、顺序和 final 标记携带历史，只能推进到 `nextTurnRequestVerified`。只有真实 provider 模型返回响应，且业务答案为 `KITE-728|琥珀色|AMBER-314`，才能推进到 `nextTurnReplyVerified`。若模型增加解释文字，语义解析可判定历史引用成功，但发布证据必须保留完整响应与所用模型。工具被调用不直接判失败，但不能把其结果当成历史已恢复的证据；测试工作区不得含这些答案。

## 6. 自动化层级

| 层级 | 范围 | 必须自动化 |
|---|---|---|
| L0 静态合同 | JSON schema、错误码、能力矩阵、禁用字段 | schema lint、fixture lint、用例清单完整性 |
| L1 单元 | extractor、final 判定、有序消息 serializer、三类身份、严格 parser、路径映射、receipt 状态机 | 所有 provider extractor 的正反 fixture；恶意包；连续 user；空 assistant/缺 assistant；崩溃点故障注入 |
| L2 组件/合同 | Tauri command/port、现有 SQLite receipt DAO 到隔离 provider adapter | SQL 事务/唯一 slot/设备绑定；无真实 HOME；验证无越界写、无旧记忆变化、错误原样传播 |
| L3 隔离 provider 进程 | 安装版 CLI/App Server + 临时 HOME | 原生导入、原生读回、打开命令、进程重启读回；无真实账号时止于可证阶段 |
| L4 单机 E2E | FyAgent UI + provider | 八个正常阶段及异常分支徽标、选择目录、重复/新快照/另存、receipt 对账；userAttestation 不升系统阶段 |
| L5 双设备 E2E | 一台 macOS 与一台 Windows | Mac→Windows、Windows→Mac，真实目标软件、真实重启和下一轮续聊 |

L5 不用同机路径字符串模拟替代。虚拟机可做预检，但发布证据必须标明硬件/VM、OS build、文件系统和 provider 版本。

## 7. Provider 能力门槛

每个 provider 单独维护以下门槛，不允许“某 provider 通过”提升其他 provider：

| Gate | 判定与最低证据 |
|---|---|
| G0 Detect | 可执行文件/服务、精确版本、目标 OS、候选导入能力探测原始输出 |
| G1 Extract | provider 合成原生日志经 final-only 提取，通过 F-GOLDEN/F-INDETERMINATE/F-INCOMPLETE/F-ORDERED-USERS |
| G2 Package import | 严格包校验后目标 adapter 完成一次隔离原生写入，记录新 native ID |
| G3 Native read-back | 通过 provider 自己支持的读取面或等价原生列表/历史接口读回，不只检查 FyAgent receipt |
| G4 Open | 对应软件实际装载该 native ID；复制命令或查看文本不算 |
| G5 Restart read-back | 完全退出并重启 provider 后再次原生读回相同历史 |
| G6 Request context | 下一轮实际请求载荷带上正确历史；模拟端点只能到此 |
| G7 Real continuation | 真实模型响应引用 KITE-728、琥珀色、AMBER-314 |
| G8 Cross-OS | Mac→Windows 和 Windows→Mac 两个方向分别完成 G2-G7 |

### 各 provider 当前进入条件

- Codex：只把 `phase=final_answer` 且 text 内容块合规的 assistant 当 Explicit；`commentary` 是 progress，缺 phase、多个 final 或 content kind 未知均阻止导出。已有模拟端点 request capture 可作 G6 证据，但 `thread/items/list` / `thread/turns/list` 为空，所以 G3 不通过，真实模型未调用所以 G7 不通过。禁止使用 schema 明示 `DO NOT USE` 的 unstable resume history。
- OpenCode：以 1.18.30 顶层 `import/export` 为首个实现版本，不能混用新版 `session import/export` 命令。先补重启、请求上下文、真实续聊、重复/新快照另存和跨 OS。
- Hermes：不得直接把现有 importer 当 final-only；必须在导入前由 FyAgent extractor 过滤并证明没有 `[ran tool: ...]` 或 commentary。
- Gemini：先验证 0.59.0 `--session-file` 的实际输入契约和 project/cwd 绑定；help 文字只满足 G0。
- Claude Code：先用合成 transcript 验证精简原生重建；没有公开导入文档记为 `unknown/unverified`，不是 `impossible`。
- Grok Build：能力检测必须以安装版为准；1.0.34 help 无 import 时返回明确不支持版本，不能调用网页文档中的命令。
- OpenClaw：先核验安装版 Gateway/存储写入入口和会话归属；只读 trajectory/历史不满足 G2。

## 8. 实际落地顺序

不做七 provider × 全 OS × 全异常的笛卡尔积，按风险收敛：

1. **合同先行**：L0/L1 跑通共享 schema、final-only、连续 user、有序序列、坏包、unknown field、大小限制、三类身份和 receipt 故障注入。
2. **SQLite 回执先行**：验证 `attemptId`、默认唯一 `idempotencySlot`、SaveAsNewCopy request 幂等、`deviceBinding/targetStoreId`、INSERT/UPDATE 事务回滚。
3. **Codex/OpenCode 架构验证**：Codex 查清原生读回空数组；OpenCode 完成 G1-G7 的单机隔离闭环。
4. **幂等与对账**：在首个通过 provider 上跑完全重复、R1→R2 显式另存、分叉显式另存、非 0/超时副作用、receipt 事务失败和进程崩溃窗口。
5. **Hermes 次验证**：复用共享合同，只补 provider 特有 extractor/native adapter 合同和 G2-G7。
6. **Gemini/Claude/Grok/OpenClaw 能力探测**：先 G0/G1；只有探测到可写契约才进入 G2，不猜测格式。
7. **UI 状态闭环**：确保八个正常阶段及异常分支不跳级，userAttestation/改目录不升 capability，未验证 provider 显示版本与缺口。
8. **跨 OS**：先选择单机 G1-G7 全绿的 provider 做两个方向，再逐 provider 扩展；每个方向独立留证。
9. **发布回归**：七 provider G0/G1、所有共享安全用例、旧记忆不变、首批宣称支持 provider 的 G2-G8。

## 9. 证据规范

每次执行保存到任务 evidence 的测试运行子目录，不写真实会话库。每个 case 至少包含：

- `case-id/result.json`：状态只能是 `passed|failed|blocked|not_run`，含开始/结束时间、git SHA、实现 build、OS、provider 版本。
- 输入 fixture 与 SHA-256；导出包与 SHA-256；递归禁用键扫描报告。
- capability probe 的 argv、exit code、stdout、stderr；凭据脱敏但不能改写错误。
- 原生写入返回、native ID、原生读回原文、重启前后进程标识。
- receipt SQL 事务边界、`attemptId/requestId/idempotencySlot/targetStoreId/deviceBinding`、目标存储前后清单/行数/hash。
- L5 的源/目标两端独立日志和方向标识。
- 旧记忆 sentinel 的路径、内容 hash、权限、mtime 前后对比。

截图只能辅助 UI 状态，不能替代机器可读读回。请求捕获必须注明端点是否模拟、`model_called`；模型续聊必须保存完整 prompt/response 和人工/机器判定依据。

## 10. 进入与退出标准

### 进入 provider G2 前

- 共享 L0/L1 全绿。
- provider G0/G1 全绿。
- 测试仅指向隔离 HOME/临时目录。
- provider 版本与导入命令已从实际探测确认。

### 标记“支持恢复”

- 对声明的 provider/版本/OS，G0-G7 全绿。
- 至少一个真实 Mac→Windows 和一个 Windows→Mac G8 记录；若某 provider 尚未完成 G8，能力文案必须限定为“单机已验证，跨 OS 未验证”，不得写“跨设备已支持”。
- 重复、新快照另存、分叉另存、SaveAsNewCopy request 幂等、receipt 事务/对账、版本不支持、目录不存在和旧记忆不变全绿。
- 未解决失败没有被降级成 warning 或空会话成功。

## 11. 本轮执行状态与依赖

本轮仅生成测试方案和结构化用例，所有 `test-cases.json` 用例均为 `not_run`。没有修改生产源码、没有调用真实模型、没有写真实 provider 会话库。

下一依赖：

1. 技术设计需与本文统一为同一套有序消息、camelCase 错误码、大小/深度限制、SQLite receipt 字段、八个正常阶段及异常分支合同；不接受只做机械改名而保留冲突行为。`SaveAsNewCopy` 同 request 重放幂等仍要求实现提供稳定 `requestId` 或等价实例键。
2. 实现提供 provider extractor 的合成输入入口、隔离根目录注入和 receipt/native write 故障点。
3. 准备 Windows 测试机及七 provider 的可核验版本；登录/配额问题按 blocked 留原始证据，不静默换模型或 provider。


协调方集成：统一 requestId，排除原始包文件 hash 参与幂等，补充重导出排版变化、同正文不同来源、未知来源 UUID、同请求副本重放与设备隔离合同。全部用例仍 not_run。

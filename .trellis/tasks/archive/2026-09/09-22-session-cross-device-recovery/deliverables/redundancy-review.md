# Session 跨设备恢复 · 独立去冗余评审

- 角色：Grok 独立架构评审（不改被审方案、不调用真实会话）
- 执行模型：grok-4.6
- 日期：2026-09-22
- 仓库基线：`codex/frontend-interaction-v3-1-20260826` @ `60d699fa00b3f275dcda556afc74a21eaa05379d`
- 本包约束：仅方案/原型/隔离验证；生产源码与真实会话库未触碰

## 0. 评审范围

已审（本轮一次评审）：

| 材料 | 路径 |
|---|---|
| 产品边界 | `research/session-recovery-20260921/恢复方案.md` |
| 中间格式草案 | `research/session-recovery-20260921/cli-formats.md` |
| 本机能力核验 | `research/session-recovery-20260921/local-cli-evidence.json` |
| Codex schema 摘录 | `research/session-recovery-20260921/codex-protocol-excerpt.json` |
| OpenCode 隔离导入 | `research/session-recovery-20260921/opencode-synthetic-result.json` + `check_opencode_import.py` |
| Hermes 隔离导入 | `research/session-recovery-20260921/hermes-synthetic-result.json` + `check_hermes_import.py` |
| 方向草案 | `.trellis/tasks/09-21-remove-client-projects/research/session-direction.md` |
| 代码可行性 | `.trellis/tasks/09-21-remove-client-projects/research/session-feasibility.md`（基于 `c0b2ec21`，部分前端结论已过期） |
| 本轮 PRD | `.trellis/tasks/09-22-session-cross-device-recovery/prd.md` |
| UX 规格 | `deliverables/ux-spec.md`（Antigravity，本轮已出现） |
| 现有读取/展示实现 | `src-tauri/src/session_manager/**`、`src/components/sessions/**`、`src/v2/pages/memory/Page.tsx` |

未出现、**必须二次审阅**（本文件不代替）：

- `deliverables/technical-design.md`（Cursor）
- `deliverables/test-plan.md` / `deliverables/test-cases.json`（Cursor）
- `deliverables/session-prototype.html`（`ux-spec.md` 第 7 节声称已交付；`deliverables/` 目录在本评审落盘时只有 `ux-spec.md`）

未运行、**不得记为已验证**：真实用户会话、模型下一轮、重启后读回、Mac↔Windows 双向、Codex `thread/inject_items` 实跑。

---

## 1. 必要性挑战：结论先行

| 被挑战对象 | 结论 | 一句话 |
|---|---|---|
| 云账号 / Codex Cloud history | **不需要，禁止** | Codex `resume.history` 标注 `FOR CODEX CLOUD - DO NOT USE`；产品排除自动云同步 |
| 持续同步（WebDAV/配置库） | **不需要，禁止复用** | 现有备份只导出 FyAgent 自己的 SQLite，不含七软件原生日志 |
| 通用任务引擎（`change_plans`/`change_jobs`） | **不需要** | 那是配置变更作业恢复，不是会话包导入 |
| FyAgent 双数据库 / 第三套 Session 表 | **不需要** | 可迁移包是 JSON 文件；去重用小型 receipt；OpenCode 自己已有 JSON+SQLite 双存 |
| 插件框架 | **不需要** | 已有 7 个硬编码 adapter；OpenCode 隔离测试刻意 `--pure` |
| 全量事件 / 隐藏原生日志迁移 | **不需要，禁止** | 与 R1 冲突；Grok `updates.jsonl` 过滤后不能当原生 resume |
| 最小重建元数据 | **需要，但不能靠展示 DTO** | 现有 `SessionMessage` 已丢掉 message id / parent / final 标记 |
| 以查看/复制/本机 Play 代替恢复 | **禁止** | v1 `SessionManagerPage` 正是这条死路；UX 里用户自点「确认续聊成功」会把它复活 |

最小充分形态（评审建议，不修改原文）：

`fyagent.session.v1` JSON 包（final-only 正文 + 重建所需最少字段）→ 目标机选目录 → provider adapter 写入**该软件**原生存储 → 独立 receipt 记录来源/目标 ID/阶段。不要新库、不要任务队列、不要云、不要插件加载器。

---

## 2. must-fix

后续技术方案若沿用下列路径，会直接违反已授权产品边界，或把已证明可行的导入打成不可恢复。

### M1. 禁止把 `get_session_messages` / `SessionMessage` 当导出源

**来源**

- DTO 只有 `role/content/ts`：`src-tauri/src/session_manager/mod.rs:31-37`，前端同构 `src/types.ts:469-473`
- OpenCode 读取时拿到了 `msg_id`，随后丢弃：`opencode.rs:181-237`（`entries: (ts, msg_id, role, text)` → `SessionMessage { role, content, ts }`）
- Codex 把 `function_call` 写成 assistant `[Tool: name]`：`codex.rs:231-257`
- Gemini 把 `toolCalls` 追加进 assistant 正文：`gemini.rs:88-97`
- OpenCode part 层同样把 `type=tool` 渲染成 `[Tool: …]`：`opencode.rs:556-569`
- 通用抽取会展开 `tool_use` / `tool_result`：`utils.rs:85-105`

**反例**

OpenCode 隔离测试之所以 `import_exit_code=0` 且 round-trip 4 条纯文本，是因为 fixture **不是** `SessionMessage[]`，而是带 `id/sessionID/parentID/model/path/finish/part.id/type=text` 的原生 JSON（`check_opencode_import.py:33-49`）。若导出坐在现有 IPC 上，得到的是「带工具名的展示文本、没有 message id」。用这份东西再生成原生文件，要么缺字段被拒，要么把 `[Tool: fake_tool]` 当最终答复写回去——后者正是 Hermes 实测里 native importer 已经犯的错（`hermes-synthetic-result.json: native_importer_final_only=false`）。

**最小改法**

读取层可复用扫描入口；导出必须有独立 final-only 投影，输出可迁移记录，而不是 `SessionMessage`。展示过滤（`shouldHideCodexMessageFromToc`，`src/components/sessions/utils.ts:210-217`）也不是导出过滤器。

### M2. `cli-formats.md` 的 `{user, assistant}` turns 不够当包格式

**来源**

- 过瘦 envelope：`cli-formats.md:17-29`
- 恢复方案已写明 OpenCode 需要消息 ID、父消息、模型等，或由**目标适配器生成**：`恢复方案.md`「要保存必要结构，而不保存执行内容」
- 产品仍要求稳定去重身份、来源软件、原始会话标识、顺序、格式版本：`恢复方案.md` 推荐路线第 2 步；PRD R3

**反例**

若包里只有 `turns[].user/assistant`：

1. 重复导入无法区分「同一来源会话」与「另一份恰好正文相同的会话」（两台机器上 `source_path` 不同，见 M4）。
2. 未完成轮次没有「有 user、无 assistant」的一等表示，适配器会伪造空答复或丢掉最后一句用户原文（与状态 4 的产品要求相反）。
3. 目标适配器若不会生成 OpenCode 合法 `parentID/finish/part`，已证明的 `opencode import` 路径会退化成未知。

**最小改法**

包格式采用「有序消息数组 + 包级身份」，不要 `{user, assistant}` 对：

- 必有：`schema`、`source_provider`、`source_session_id`、`migration_id`（包稳定 ID，与目标原生 ID 解耦）、有序 `messages[{role, text, turn_index, complete}]`、`exporter_cli_version`
- 可选/由目标适配器填：目标 message id、parent、model、原生 path
- 禁止：tool/thinking/progress/files、凭证、`source_path`、`resume_command`

「最少元数据」指重建与去重所需字段，不是把原生事件整包塞进 JSON。

### M3. 现有 Session 页的复制 / Play 不是恢复；UX 的「确认续聊成功」会把它产品化

**来源**

- v1 已有 `SessionManagerPage`：复制消息、Windows 只复制 resume 命令、macOS `launch_session_terminal`（`SessionManagerPage.tsx:393-444`；`App.tsx:874-879`）
- 终端拉起只支持 macOS：`terminal/mod.rs:28-30`
- OpenClaw / Hermes 无 resume command：`openclaw.rs:298`，`hermes.rs:146`
- 用户已否决「推荐只做查看/复制」：`恢复方案.md`「对早期研究意见的裁定」；`session-direction.md`「已解决的方向选择」
- UX 仍把恢复闭环做成用户回 FyAgent 点「确认续聊成功」（`ux-spec.md` §4.4、状态 6）
- UX 四阶漏斗把「重启后读回」和「Mac/Windows 双向」并进「续聊验证」（`ux-spec.md` §1.3），少于 PRD 六级验收

**反例**

用户在目标软件里只打开历史、复制一段答复、或甚至什么都没发，回到 FyAgent 点「验证续聊成功」，徽章变成「已恢复 · 续聊正常」。这与「查看/复制不算恢复」字面冲突。Windows 上现有 Play 路径本来就是复制命令（`SessionManagerPage.tsx:421-426`），更容易把「已复制」显示成「已打开」。

**最小改法**

- 列表/详情/复制可以存在，但状态机不得有「用户勾选即 Verified」。
- 六级分开记：包导入、原生读回、打开、重启后读回、下一轮引用合成事实、Mac↔Windows。任一级未跑标 `unverified`，禁止用相邻级冒充。
- 「打开」在 Windows 未验证前，按钮文案只能是「复制启动命令」，不能叫「已在目标软件打开」。

### M4. 去重身份不能用 `source_path`，也不能提供覆盖

**来源**

- `getSessionKey` = `providerId:sessionId:sourcePath`（`src/components/sessions/utils.ts:69-70`）
- OpenCode SQLite 引用是本机路径：`sqlite:{db}:{ses_…}`，Windows 路径含冒号（`opencode.rs:77-87,149`）
- PRD R3：导入去重、**不覆盖**已有原生会话
- UX 状态 5 提供「覆盖更新（谨慎）」（`ux-spec.md` 状态 5）

**反例**

同一 OpenCode 会话从 Windows 迁到 Mac：源 ID 相同，`source_path` 从 `sqlite:C:\Users\…` 变成 `sqlite:/Users/…`。用现有 `getSessionKey` 会当成两个会话，重复写入；若改用源 ID 直接当目标 ID，OpenCode「按 ID 写入、有冲突处理」（`恢复方案.md`）会打到已有行上。UX 的覆盖选项把这条变成用户可点的破坏操作：导入对话框打开期间目标机若已有新回复，覆盖会删掉那轮对话。

**最小改法**

稳定键 = `source_provider + source_session_id + package_hash`（或 `migration_id`）。命中已有映射 → 默认跳过或「另存新原生 ID」。删除覆盖选项。目标原生 ID 允许变，映射写在 receipt 里，不写回包文件当唯一 ID。

### M5. 禁止把「换一个好导入的软件」当成恢复降级

**来源**

- 产品：目标对应软件原生续聊；七软件是范围，Codex/OpenCode 只是验证顺序（PRD R5；`恢复方案.md` 实施顺序）
- UX 状态 2：「转换为支持导入的客户端（如 Codex / OpenCode）」（`ux-spec.md` 状态 2）

**反例**

本机 Grok 1.0.34 没有 `import`（`local-cli-evidence.json` capabilities.grok）。若 UX 引导「先转到 OpenCode」，用户看到的是另一款软件里的问答副本，不是 Grok 续聊。这是把查看/复制换皮成跨软件粘贴，且会把产品范围永久缩小到「谁碰巧能导入」。

**最小改法**

版本不支持 → 明确失败 + 复制文本（标明「这不是恢复」）。跨软件重建是另一个产品，本包不做。七软件各自独立记能力，未验证不标支持、不标不可能。

### M6. 路径重映射不得改写用户原文；「不含绝对路径」与实测相反

**来源**

- OpenCode 导入后 `directory` 被改到目标 cwd，但 assistant `path.cwd` 仍是合成的 `C:\synthetic\old-workspace`（`opencode-synthetic-result.json:15-16`；`check_opencode_import.py:27,41,68`）
- 恢复方案：处理恢复元数据里的路径，**不任意改写用户消息原文**
- UX 状态 7：「不会将旧系统路径混入新的对话上下文」；导出「绝不含绝对敏感路径」（`ux-spec.md` §4.2、状态 7）

**反例**

用户原文是 `请看 D:\work\app\bug.ts 第 12 行`。若导入时按「隔离绝对路径」改写成 `/Users/me/app/bug.ts`，目标模型看到的不是用户说过的话，下一轮引用会错。若 adapter 删除 assistant 的 `path` 字段而 OpenCode 1.18.30 需要它，导入会从「已证明」退回「未知」。

**最小改法**

只映射会话级目录元数据（选目标工作区）。消息正文原样保留。adapter 需要的原生 `path/cwd` 在写入时用目标目录生成，不要从用户文本里「清洗」路径。包内可以带着源逻辑 workspace 字符串，仅供 UI 提示「请选择本机对应目录」。

### M7. 官方 importer / 顶层 backup 不能当 final-only 通道

**来源**

- Hermes `sessions import` 保留 commentary 和 `[ran tool: fake_tool]`，丢掉 tool output（`check_hermes_import.py:50-54`；`hermes-synthetic-result.json:13-14`）
- 本机 Hermes 顶层 `hermes import` 是全量 backup，超出 final-only（`local-cli-evidence.json` capabilities.hermes）
- Codex `resume.history`：`[UNSTABLE] FOR CODEX CLOUD - DO NOT USE`（`codex-protocol-excerpt.json:10-14`）
- Grok 权威日志是 `updates.jsonl`，过滤事件会破坏 ACP 序列（`cli-formats.md` Grok 节）

**反例**

把 mixed Codex 日志丢给 Hermes 官方 importer，读回历史里会出现「Checking now.」和工具名短注——这正好是产品要排除的进度/工具。把 Codex Cloud `history` 当跨设备手段，等于引入云账号。把瘦身 `updates.jsonl` 写回 Grok，是「为了原生 resume 而偷偷做全量事件、结果还是坏文件」。

**最小改法**

FyAgent 先投影 final-only，再让 adapter 生成目标原生结构。官方 importer 仅在投影之后、且实测不再夹带 commentary/tool 标记时才能用。禁止：Cloud history、Hermes 全量 backup、Grok 过滤后的 `updates.jsonl` resume、隐藏完整 JSONL 夹带。

---

## 3. simplify

后续设计若加入下列层，属于冗余，应直接砍掉。

### S1. 不要新建 FyAgent Session 数据库

现有 `Database` 已有配置库 + WebDAV 同步跳表（`backup.rs:75-98`）。会话文件不在这些表里（`session-feasibility.md`「前端与跨设备导出」）。再加 `sessions`/`session_messages` 表会立刻诱使「跟配置一起 WebDAV 同步」——即被排除的持续同步。

崩溃窗口用 sidecar receipt 足够：`migration_id`、阶段、目标 provider、目标原生 ID、目录、错误。不要第二份消息正文库。

### S2. 不要把导入接进 `change_plans` / `change_jobs`

`schema.rs` 已有通用作业表，WebDAV 还把它们当本机保留表（`backup.rs:81-97`）。会话导入是一次性文件适配，不是可恢复的多步配置作业。接进去会得到：队列、event_seq、跨设备作业同步、权限模型——全部与「U 盘拷一份 JSON」无关。

### S3. 不要插件框架；7 个 adapter 保持硬编码

扫描入口已经并行 7 路（`session_manager/mod.rs:58-66`）。OpenCode 验证用 `--pure` 关掉外部插件（`check_opencode_import.py:52`）。Session 恢复的差异在各 CLI 版本契约，不在第三方插件。做成 plugin ABI 只会把「版本检测」换成「插件加载失败」而无新证据。

### S4. UX 生产界面不要带「状态自检切换栏」

`ux-spec.md` §3.3 TopBar 把原型用的状态切换写进信息架构。那是评审夹具，不是用户功能。生产页只要真实状态徽章。八态切换留在原型。

### S5. 旧记忆不要改成只读抽屉，也不要做指纹服务

`ux-spec.md` §2.2 把旧记忆做成「轻量只读/查看抽屉」。现有 v2 记忆页是在编辑 OpenClaw/Hermes 真正消费的 `MEMORY.md`/`USER.md`（`src/v2/pages/memory/Page.tsx:104-138`）。产品定位审阅已说明：编辑目标软件实际消费的文件仍是核心价值（`product-positioning-review.md`）。最小安置 = 二级入口、可编辑、不混进会话列表、不删除文件。

「源会话特征指纹」若做成独立服务/库，与 M4 的 `provider+source_id+package_hash` 重复。不要为去重再引入内容指纹集群。

### S6. 不要为了 native resume 保留 tool 短注「以防万一」

Hermes 官方路径证明：短注会被读回。产品要的是最终答复原文，不是「几乎 final-only」。缺最终答复就显式失败（PRD R1），不要用 `[ran tool: …]` 填洞。

---

## 4. keep

这些基线决定已经去过冗余，后续方案应保留，不要再打开。

| 保留项 | 依据 | 不要改成 |
|---|---|---|
| Session 替换记忆一级入口；记忆文件不删、不改成会话 | PRD R2；`恢复方案.md` 产品决定；`navigation.ts` 仍是 memory 一级 | 只改标签、或自动迁移 MEMORY.md |
| 每轮用户原文 + 最终答复原文，不摘要 | PRD R1 | LLM 浓缩、只留最后一条 assistant |
| 排除工具/进度/思考/附件/工作文件 | PRD 排除；`恢复方案.md` | 隐藏完整 JSONL「以便续聊」 |
| 目标对应软件、原生 ID 可变 | PRD R3；`恢复方案.md` | 跨设备必须保留同一 ID |
| 用目标机登录与模型，不迁认证 | PRD R4 | 打包 token、Cloud history |
| 七软件产品范围；Codex/OpenCode 只是验证顺序 | PRD R5 | 把未验证的五家标成永久不做 |
| 无公开导入文档 ≠ 不可能 | `恢复方案.md` 裁定 | 文档缺失就写死 unsupported |
| 配置备份 / WebDAV ≠ Session 迁移 | `backup.rs`；feasibility | 把会话塞进 SQL dump |
| 人工文件拷贝（U 盘/隔空投送），无自动云同步 | `session-direction.md`；UX §4.2 | 常驻同步队列 |
| 隔离合成测试、不读真实会话 | 两份 synthetic-result.json | 对着 `~/.codex`/`~/.opencode` 写回 |

可行性文档有一处过期，引用时注意：它写「`src/` 未发现调用端」（`session-feasibility.md` 前端节），当前工作区 v1 已有完整 `SessionManagerPage`。结论「现有读取 ≠ 跨设备恢复」仍然成立；「没有 Session UI」不再成立。二次审阅不要基于这篇的前端段落做范围裁剪。

---

## 5. evidence-needed

未知一律标缺口，不许写成支持或不可能。

| ID | 缺口 | 已有证据停在哪 | 若无证据禁止写什么 |
|---|---|---|---|
| E1 | Codex `thread/inject_items` 注入 user/final assistant 后：持久化、UI 读回、重启、下一轮引用 | schema 描述「会进入后续模型请求」（`恢复方案.md` 表；`codex-protocol-excerpt.json` inject_items）；**未实跑** | 「Codex 已支持恢复」 |
| E2 | OpenCode 导入后模型下一轮、重启、Mac↔Windows | 仅隔离 import/export 文本 round-trip | 「OpenCode 端到端恢复完成」 |
| E3 | Gemini `--session-file` 是否接受 final-only JSON、是否绑定 project hash | 本机 help 有 flag（`local-cli-evidence.json`）；官方文档历史含 tool IO（`cli-formats.md`） | 「生成兼容文件即可 resume」 |
| E4 | Claude 精简 transcript 能否被本机 `--resume` 当历史 | 有 resume/fork；无任意精简导入契约 | 「不可能」或「已支持」 |
| E5 | Grok 1.0.34 有无 import；`chat_history.jsonl` 能否代替 `updates.jsonl` | 网页有 import，本机 help 无；现有读取走 `chat_history.jsonl`（`grokbuild.rs:54-90`） | 把当前 reader 当写入器 |
| E6 | OpenClaw 精简历史可写入口 | 有 list/export-trajectory；无通用 import；`resume_command=None` | 只读共享/轨迹导出 = 恢复 |
| E7 | Hermes 自建 Session（绕过官方 importer）后续模型回复 | 官方 importer 非 final-only；离线 append 用户消息已过 | 「Hermes 已可续聊」 |
| E8 | 各 adapter 重建字段的最小集合（OpenCode `finish/parentID/tokens` 是否可生成） | 一份 1.18.30 成功 fixture，未做字段消融 | 把整份原生 export 当「最少元数据」 |
| E9 | Windows「打开」：无 macOS `launch_terminal` 时何谓通过 | `terminal/mod.rs:28-30` 直接拒绝非 macOS | UX 统一「启动终端并继续」且标已打开 |
| E10 | UX 所称 `session-prototype.html` 与 Cursor 技术方案 | 本评审时文件不在 `deliverables/` | 把 UX 文案当已实现契约 |

Codex 用户消息里的 IDE 注入（`# Context from my IDE setup:`，`utils.ts:5-67`）是「用户原文」还是「运行时注入上下文」：恢复方案要求区分。导出时若整段保留，会迁入环境上下文；若只用 `extractCodexPromptPreview`，可能裁掉用户原文。需要带合成 fixture 的明确规则，不要沿用 TOC 预览函数。

---

## 6. 最小元数据：删了会坏 vs 必须删

| 字段/内容 | 处置 | 反例 |
|---|---|---|
| 有序 user / final assistant 正文 | 保留 | 只留最后一条 assistant = 摘要替代原文 |
| `complete=false` 的用户轮 | 保留为空答复槽，禁止补写 | 填一句假「已中断」会变成伪造最终答复 |
| `source_provider` + `source_session_id` + `migration_id` | 保留 | 丢掉后无法去重，只能覆盖或双份 |
| 包格式版本 + 源 CLI 版本 | 保留 | 否则「版本不支持」无法诚实失败 |
| 会话标题、时间、轮次序号 | 保留 | 列表与验收对顺序 |
| 逻辑 workspace 提示（非正文） | 保留为提示，目标机重选 | 当唯一恢复凭据会在跨 OS 失败 |
| 目标侧 message id / parent / model / path | **adapter 生成**，不要当跨设备主键 | OpenCode fixture 需要它们，但不需要等于源 ID |
| `source_path` / `resume_command` | **删除出包** | 本机 SQLite URI、macOS 专用 Play |
| tool/function_call/tool_result/progress/thinking | **删除** | Hermes `[ran tool:]`、Codex `[Tool:]` 展示文本 |
| 工作区文件、附件、token、Cloud history | **删除** | `hermes import` 全量 backup；Codex DO NOT USE history |
| 用户正文里粘贴的路径/代码 | **保留原文** | 当敏感路径清洗会改写用户话 |

原则：怕不可恢复就保留**身份与顺序**；怕违反 R1 就删除**执行内容**。不要用「留着 tool 以免 resume 失败」同时打穿两条。

---

## 7. 「查看替代恢复」现成入口（实施时会踩）

1. **v1 Session 页**：复制 + Play。Windows 复制命令即返回（`SessionManagerPage.tsx:421-426`）。把 memory 一级入口换成这一页而不加写入/六级状态，就是被否决的早期建议。
2. **现有 IPC 名称**：`list_sessions` / `get_session_messages` / `launch_session_terminal` 读起来像恢复 API，实际是本机索引 + macOS 拉起（`commands/session_manager.rs:4-86`）。技术方案若「复用现有命令做跨设备恢复」而不加新命令边界，会在文档里把查看写成恢复。
3. **UX 用户自证**：§4.4「确认续聊成功」。见 M3。
4. **UX 跨软件转换**：状态 2。见 M5。
5. **Grok/OpenClaw 只读**：`chat_history.jsonl` 能展示；`export-trajectory` 能看轨迹。两者都可以做很好的详情页，都不能标「已恢复」。

验收用语必须保持分裂：`imported` ≠ `native_visible` ≠ `opened` ≠ `restart_visible` ≠ `next_turn_cited` ≠ `cross_os`。

---

## 8. 二次审阅清单（Cursor 方案 / 测试 / 原型落地后）

不阻塞本文件。下一依赖出现后按此对读，不必重写基线节。

1. 包 schema 是否仍是 `{turns:[{user,assistant}]}`（M2）还是有序消息 + `migration_id`。
2. 导出是否调用 `get_session_messages`（M1）。
3. 是否新增 Session 表、change_job、WebDAV、plugin registry（S1–S3）。
4. 去重是否用 `source_path` 或提供覆盖（M4）。
5. 是否把 `launch_session_terminal` 当 Windows「打开」通过条件（E9）。
6. 是否为 Grok/Claude/OpenClaw 写了无证据的「支持」或「不可能」。
7. 测试是否六级分证，是否包含：final-only、坏包、未知字段夹带、未完成轮、重复导入、receipt 失败但原生已写、旧记忆文件不变。
8. 原型是否真的在盘上，且未把假数据标成真实恢复。
9. Hermes/Codex 路径是否仍走官方 importer/Cloud history（M7）。
10. 路径映射是否改写消息正文（M6）。

---

## 9. 自检

- 只写了本角色文件 `deliverables/redundancy-review.md`；未改 `恢复方案.md`、`ux-spec.md`、生产源码。
- 未读、未写真实会话库；未调用模型。
- 未配置新凭据；无 Jev 请求（取舍已能用现有证据闭合，不代替测试）。
- 工作区 dirty 界面未动。

### 回执

| 项 | 值 |
|---|---|
| 产物 | `.trellis/tasks/09-22-session-cross-device-recovery/deliverables/redundancy-review.md` |
| 执行模型 | grok-4.6 |
| 阻塞 | `technical-design.md`、`test-plan.md`/`test-cases.json`、`session-prototype.html` 未齐，需二次审阅；E1–E10 未跑 |
| 下一依赖 | Cursor 技术方案必须先消化 M1–M7 / S1–S6，再写测试；主协调在三份方案齐后派本角色补第二轮对照，不要把本轮基线评审当成对未出现文件的签字 |

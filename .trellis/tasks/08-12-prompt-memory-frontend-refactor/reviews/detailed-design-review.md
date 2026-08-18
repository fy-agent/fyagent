# Prompt / Memory 详细设计静态评审

- 评审阶段：C（详细设计与执行计划）
- 评审日期：2026-08-12
- 证据等级：`code_audit`（仅静态阅读）
- 主评审对象：`detailed-design-overview.md`、`execution-plan.md`
- 对照范围：`prd.md`、`technical-design-overview.md`、`reviews/product-design-review.md`、`reviews/technical-architecture-review.md`、`design.md`、`implement.md`
- 执行边界：未运行 lint、typecheck、任何 unit/integration/browser test、Playwright、build、dev server、截图或 pixel diff
- 写入边界：仅新增本评审文档；未修改任务状态、设计、源码、测试、规范或其他评审文档

## 1. 结论

`DETAILED_DESIGN_REVIEW=PASS`

总体方案已经收敛：三个执行 Agent 的文件 owner 互斥，Prompt / Memory 的页面状态留在各自模块，shared 只承担目标合同与 standalone，集成被明确排在三个模块及各自单测之后；设计也明确保护 `src-tauri`、Agent / models / Skills / MCP、navigation、router、AppShell 与 V2 Shell，没有发现 P0，也没有发现要求大范围重写或恢复旧产品模型的内容。

关闭复审确认原 3 个 P1 已全部进入可执行设计：standalone 新行为有独占测试文件、可测试边界与双文件模块命令；Prompt 开关采用原子 saved/draft/baseline 转换并覆盖 clean/dirty 两类用例；Git 保护审计同时覆盖 baseline-to-HEAD、worktree/untracked、逐提交清单和冻结文件 hash。原 2 个 P2 和 1 个 P3 也已写入 Memory 模块与最终 standalone 验收条件。

本次未发现新的 P0/P1。详细设计可进入整体设计冻结；该 PASS 只证明设计和执行门禁闭合，不代表实现或运行验收已经完成。

## 2. 逐项核对

| 核对项 | 静态证据 | 结论 |
| --- | --- | --- |
| 文件所有权互斥 | `detailed-design-overview.md:43-93`、`execution-plan.md:91-160` 将 Prompt、Memory、shared/standalone、主 Agent 文件分别列为独占；页面测试也落在各自模块 | 通过 |
| 三执行 Agent 不并发改同文件 | 三线路 owner 无交集；shared 签名在设计中冻结，页面 Agent 在 shared 完成前不得越权改 shared 或提前跑单测（`execution-plan.md:162-165`） | 通过 |
| 状态转换与失败路径 | Prompt / Memory 均有事件表、dirty guard、校验、放弃和 route blocker；Prompt 开关原子转换、Memory clean save / promote / revision-task 失效均已定义 | 通过 |
| 每个行为有模块单测 | Prompt、Memory、shared target 与 standalone builder 均有独占聚焦测试及唯一模块命令 | 通过 |
| 集成严格在全部模块及单测之后 | `execution-plan.md:166-209` 要求三个 Agent 返回、三条模块命令通过和 owner 核验后才首次运行 lint/typecheck/full unit/build/browser | 通过 |
| 无隐性后端 / Shell / navigation / router 改动 | `detailed-design-overview.md:553-569` 明确禁止相关实现改动；route guard 只落在两个页面；future port 仅留在文档 | 通过 |
| 不碰其他人负责的 Agent / models / Skills / MCP | 详细设计、执行 Agent 共同提示和最终保护审计均明确排除四个模块 | 通过 |
| 无重复实现或大范围重写 | 采用页面内 baseline + 窄 shared 合同；不新建全局 store、service、假 adapter、通用表单框架或跨页组件库 | 通过 |
| standalone 承诺准确 | 承诺限定为当前 production 静态 entry graph，明确不支持未来未知 dynamic graph；模块测试覆盖解析/fail-fast/path escape，最终覆盖 `file://`、资源残留和 console/page error | 通过 |
| Git 多提交/推送不误纳无关改动 | 显式 path 小提交、禁止 `git add -A`；baseline-to-HEAD、worktree/untracked、逐提交 name-only 和既有受保护文件 hash 共同审计 | 通过 |

## 3. P0

无。

## 4. P1 关闭记录

### P1-01 standalone 新行为没有模块级单测设计（已关闭）

- 证据：standalone owner 要修改 `scripts/build-v2-preview.mjs`，新增从 `dist/index.html` 解析全部直接 module scripts / stylesheets、保持顺序、无 entry fail-fast、拒绝 `../` 逃逸以及停止按体积猜入口等行为（`detailed-design-overview.md:435-455`）。
- 关闭证据：`tests/v2/scripts/build-v2-preview.test.ts` 已加入线路 3 独占 owner；builder 明确导出无副作用的解析、路径解析和 build 函数，fixture 只使用临时目录；线路 3 唯一命令同时运行 shared 与 builder 两个测试文件（`detailed-design-overview.md:63-70,461-465,508-525`；`execution-plan.md:140-164`）。用例覆盖无 entry fail-fast、多 stylesheet 顺序、全部直接 module scripts 与 path escape。
- 关闭结论：设计层已满足模块先验收、全部模块绿后才集成。
- 是否阻断：**否，已关闭**。

### P1-02 Prompt 启用开关与 saved baseline 的转换未闭合（已关闭）

- 证据：dirty 定义把 `enabled` 与 baseline 比较（`detailed-design-overview.md:183-194`）；但 `TOGGLE_ENABLED` 又被设计为立即更新 saved item 和当前 draft（`detailed-design-overview.md:196-213`），没有说明 baseline 是否以及如何更新。技术概要把它定义为 saved item 的独立切换（`technical-design-overview.md:235-240`）。
- 关闭证据：`TOGGLE_ENABLED` 已锁定为 saved rule 即时前端提交，原子更新 saved item；当前项只同步 `draft.value.enabled` 与 `baseline.enabled`，不覆盖其他 draft/baseline 字段。测试清单覆盖 clean 切换不 dirty，以及其他字段已 dirty 时切换不丢草稿、放弃仍保留已提交开关（`detailed-design-overview.md:196-215,469-486`；`execution-plan.md:100-112`）。
- 关闭结论：saved、draft 与 baseline 只在 enabled 维度同步，dirty/discard 语义唯一。
- 是否阻断：**否，已关闭**。

### P1-03 提交后的保护范围审计无法发现已提交误纳（已关闭）

- 证据：执行计划会在设计、三个模块和集成阶段多次提交并逐次推送（`execution-plan.md:57-65,166-184,264-284`），但实施前与最终保护命令都只使用 `git diff --name-only -- src-tauri`（`execution-plan.md:67-76,245-262`）。裸 `git diff` 默认只看尚未提交的工作区差异。
- 关闭证据：最终审计增加 `e33d37dd6f9d58c11207f843b5c33750a79dbb4a...HEAD` 累计提交检查，同时使用普通 diff/status 检查 worktree 与 untracked；受保护范围覆盖后端、其他四页、navigation、router、AppShell 和两个无关图片目录。每个小提交用 `git show --name-only` 回读 owner 清单；既有受保护文件的冻结前 Git blob hash 已实际记录在 `research/pre-implementation-protected-hashes.md`，最终逐项复算比对（`execution-plan.md:245-265`）。该 hash 记录明确只是 `code_audit`，未运行测试、构建、服务或浏览器。
- 关闭结论：已提交、未提交、untracked 与冻结文件漂移均有独立证据，不再用空 worktree diff替代分支审计。
- 是否阻断：**否，已关闭**。

## 5. P2 关闭记录

### P2-01 Memory 的无变化保存与提炼后选中态需写成确定转换（已进入验收）

- 证据：`SAVE` 表无 `isDirty` 前置并写“revision + 1、清空旧 tasks”（`detailed-design-overview.md:327-347`），同节随后又限定只有标题/正文/目标形成新 revision 时才递增。`PROMOTE` 创建 long-term transient，但没有逐字段写明随后切到 longTerm、选中新项、`baseline=null`、`localState=changes-pending` 与初始 target 集合。
- 关闭证据：clean save 已锁定为 disabled/no-op，不增 revision、不清 tasks；`PROMOTE` 原子切至 longTerm、选中新 transient、设置 `baseline=null`、`changes-pending`、空目标和完整 provenance。两者均进入 Memory 聚焦测试（`detailed-design-overview.md:327-351,488-506`；`execution-plan.md:114-138`）。
- 是否阻断：**否；作为 Memory 模块验收条件**。

### P2-02 Memory 查询与分类状态保持缺少显式模块断言（已进入验收）

- 证据：PRD 要求分类和来源切换保留各自搜索与选中项（`prd.md:180-187`）；页面状态已设计为 per-category `queries` 与 `selectedIds`（`detailed-design-overview.md:292-306`），但 Memory 单测清单没有明确验证搜索过滤、空结果以及分类往返后 query/selection 保留（`detailed-design-overview.md:476-494`）。
- 关闭证据：Memory 测试清单明确覆盖分类往返后每类 query/selected item 独立保留，以及无结果空状态不清除 saved/selected state（`detailed-design-overview.md:494-506`；`execution-plan.md:123-138`）。
- 是否阻断：**否；作为 Memory 模块验收条件**。

## 6. P3

### P3-01 standalone 最终回读应同时检查外部 entry 资源残留（已进入验收）

- 证据：详细设计承诺生成物不留下 `dist/assets` entry script/stylesheet 请求（`detailed-design-overview.md:437-455`），运行验收只写“`file://` 可直接打开并切两页”（`execution-plan.md:207-224`）。
- 关闭证据：最终 browser/readback 已要求 `file://` 可用、生成 HTML 不残留本地 `dist/assets` entry script/stylesheet，并验证 console/page error 为空（`detailed-design-overview.md:527-546`；`execution-plan.md:211-233`）。承诺仍限定为当前 production 静态 entry graph。
- 是否阻断：**否；作为最终运行验收条件**。

## 7. 已确认无需扩大范围的设计结论

1. Prompt、Memory、shared/standalone 三条线路的源码与模块测试文件当前没有交叉 owner。
2. 两个页面只消费冻结的 shared 签名；等待 shared 稳定后再跑各自模块测试，不需要修改 router、navigation 或 Shell。
3. Daily/Session 只读、提炼 provenance、逐目标 pending preview task、durable `not-run`、revision 失效和 route dirty guard 均已有明确页面落点。
4. `promptCanonicalResourceKey` 只服务 Prompt；Memory 继续按 4 个 verified adapter/scope group，不用 Prompt 路径猜 Memory 文件身份。
5. 不需要新建 backend port 实现、全局 store、service/container、shared hook、通用表单框架或跨页面组件。
6. standalone 承诺已合理限定为当前 production 静态 entry graph；修订只需要补模块测试与最终残留引用验收，不需要改 router。
7. 其他人负责的 Agent 目录、模型、Skills、MCP，以及 `src-tauri`、navigation、router、AppShell、V2 Shell 实现均应继续保持零改动。

## 8. 关闭复审结论

1. 原 P1-01、P1-02、P1-03 均已关闭。
2. 原 P2-01、P2-02 与 P3-01 均已进入对应模块或最终验收条件。
3. 文件 owner 仍互斥；没有新增后端、Shell、navigation、router、Agent / models / Skills / MCP 改动，也没有新全局抽象或大范围重写。
4. 三条模块命令全部新鲜通过且主 Agent 完成 owner 静态核验之前，仍不得开始完整 lint/typecheck/unit/build/browser 集成。
5. 本轮结论为 `DETAILED_DESIGN_REVIEW=PASS`，可以参与整体 `DESIGN_REVIEW=PASS` 与设计冻结判定。

整体设计冻结仍须同时满足产品设计复审已关闭原 P0/P1、`ARCHITECTURE_REVIEW=PASS`，以及入口文档对三份权威设计/计划没有旧 executable shape 残留。

## 9. 实施后证据回读（2026-08-13）

- 三个独占模块按 owner 分批完成；最终状态边界复核发现的 Prompt 跨条目开关、Memory trim-only revision 与 Daily/Session browser coverage 缺口均已修复。
- exact Node 24.19.0 下 lint、typecheck、12 files / 82 unit tests、161 modules build、48 browser tests 全部通过。
- 两张 1586×992 图片只作为 `runtime_screenshot`；未运行 `pixel_diff`。详细设计评审结论维持 `DETAILED_DESIGN_REVIEW=PASS`。

# Stage 5 — 前端交互与状态架构可靠性治理

## Goal

在保留当前 V2 业务能力和视觉方向的前提下，修复用户已遇到的选中态变暗/漂移问题，并系统治理以下架构风险：

- 语义选中状态依赖异步 `SelectionLens` 浮层；
- 自研 Tabs 交互不完整，且没有复用已安装的 Radix Tabs；
- 六个一级页面静态导入，访问后永久挂载；
- Models 页面再次实现一层永久挂载，并在 render 阶段更新 state；
- 隐藏页面的 query、observer、effect 和 DOM 生命周期缺少统一约束；
- Search / Settings / Account 是可聚焦、可点击但无行为的空控件；
- Skills/MCP 写入采用全局 single-flight，却让其他开关保持可点击并静默吞掉点击；
- 部分大型页面、CSS 和状态控制器职责过密，公共能力没有在真实第二消费者出现时及时提升；
- 测试虽然通过，但存在大量 React `act(...)` warning，降低交互时序证据可信度；
- 首屏主 chunk 已触发 Vite 大包警告。

本任务不是重新设计全部前端，也不是为拆文件而拆文件。它优先修复可靠性和状态所有权，再按真实复用关系提取共享组件/控制器。

## Requirements

### 1. Correct the authoritative frontend specs first

- 修订 `.trellis/spec/frontend/v2-shell.md` 中以下过度绑定实现的条款：
  - “所有访问过的一级页面在整个 renderer session 永久挂载”不再是默认合同；
  - `SelectionLens` 的具体 observer/动画实现不再承担语义选中态；
  - 页面状态保存改为按业务草稿/资源类型显式声明，而不是由 blanket keep-alive 隐式获得。
- 更新 `state-management.md`、`reuse.md`、`quality-guidelines.md`，明确：
  - 选中/激活状态由元素自身 CSS + ARIA 保证；
  - 动画/玻璃层只能增强，不得是唯一状态表达；
  - route code splitting 与 inactive-query policy；
  - 测试 warning 不能被静默过滤；
  - 新公共组件必须有一个稳定语义和真实第二消费者。
- 先更新合同或与首个实现提交同时更新，不能让代码修复反而违反旧 SPEC。

### 2. CSS-first semantic selected state

- SideNavigation、FeatureTabs、CatalogList 等选中宿主必须自身具有稳定、可读的：
  - 背景或左侧/边框指示；
  - 文本/图标对比度；
  - focus-visible 表达；
  - `aria-current` / `aria-selected` / `data-state` 等语义。
- `SelectionLens` 仅作为 `pointer-events: none` 的装饰层；以下情况选中态仍必须清晰：
  - Lens 未挂载；
  - `ResizeObserver` 不存在；
  - observer 回调延迟；
  - hidden surface 切换；
  - reduced motion；
  - WebView/backdrop-filter 不可用；
  - 用户点击右侧任何控件后。
- 删除选中宿主 `background: transparent; border-color: transparent; box-shadow: none` 作为唯一基础态的做法。
- 将 Lens observer 限定为活动 host 和必要 track/container；不得递归观察整个布局子树的每个元素。
- 对位置变化只在必要的 layout/resize/active-host 变化时测量；避免 MutationObserver + subtree ResizeObserver 引发级联布局读取。
- 若导航、Tabs、Catalog 的 selected treatment 共享同一视觉语义，提升为 `src/v2/shared/ui` 的统一 CSS/小组件 owner；若语义不同，共享 token 和基础 recipe，不强行合并 JSX。

### 3. Replace hand-written Tabs behavior with the adopted primitive

- 保留 `FeatureTabs` 作为 FyAgent 内部共享 API，内部迁移到已安装的 `@radix-ui/react-tabs`。
- 页面继续依赖 `FeatureTabs`，不在各页面直接导入 Radix 并形成多套包装。
- 支持完整键盘语义：ArrowLeft/Right（按方向）、Home、End、roving tab stop、focus/activation 策略。
- 每个 Tab 与对应 TabPanel 建立稳定 ID、`aria-controls` / `aria-labelledby` 关系；不可见 panel 的隐藏/卸载策略由调用方显式选择。
- `FeatureTabs` 继续使用 FyAgent token、selected treatment 和 reduced-motion policy。
- 迁移 Skills、MCP、Prompts、Memory、Agent configuration 等真实消费者，删除页面级平行 tab recipe。

### 4. Route loading and state ownership

- 六个一级 route 改为 route-level `React.lazy`/dynamic import；初始打开一个 route 不加载其余五个页面模块。
- `PersistentPrimaryOutlet` 不再在 render 期间调用 `setState`，也不再默认永久挂载所有 visited pages。
- Models 页删除 render-phase `setSessionTarget` / `setVisitedTargets`；route/query parameter 是当前 target 的权威来源。
- 页面离开后的状态按类别处理：
  - backend resource：TanStack Query/cache owner；
  - URL 可表达的非秘密选择：hash query parameter；
  - 未保存业务草稿：route/domain-owned draft controller，明确生命周期和 dirty blocker；
  - 临时视觉状态：允许卸载丢弃；
  - secret：只保留在当前必要内存，离开/失败时按现有安全合同清理。
- 不新增全局 Zustand/Jotai/万能 Context；只有跨树且稳定的状态才进入 shared provider。
- 若某个页面确实需要 keep-alive，必须有测量证据、明确状态理由、内存/查询生命周期和独立测试；不得继承“访问过即永久挂载”。
- `PersistentSurface` 仅保留给经过审查的窄场景，或完成弃用/重命名以防继续被当作默认路由方案。

### 5. Inactive queries, effects and observers

- 所有 V2 query/hook 接受或派生 `enabled/active`；隐藏或未激活页面不自动 fetch/refetch/poll。
- 退出 route 时清理 event listener、MutationObserver、ResizeObserver、timer、subscription 和 pending visual work。
- Auth/install 等需要跨 route 持续的 backend job 由 backend/query owner维持，页面只订阅；不能靠隐藏页面永久挂载维持任务。
- 添加测试证明：
  - 未访问 route 不创建其 query；
  - 离开 route 后不继续页面级 polling；
  - 返回 route 时从 cache/backend authoritative state 恢复；
  - dirty draft 的离开策略符合其 domain contract。

### 6. Honest shell controls

- Search、Settings、Account 三个工具按钮逐一做产品决策：
  - 已有真实 surface/route/command 时接入；
  - 当前无功能时从生产 shell 移除；
  - 只有确实需要展示未来能力时才使用明确 disabled + 非聚焦/说明状态。
- 禁止可点击 `noop`、空 handler、只打日志或无可见结果的生产控件。
- Shell keyboard order、tooltip 和 accessible name 随真实控件集合更新；测试不再把空按钮当作“可达即通过”。

### 7. Authoritative assignment mutations

- Skills/MCP assignment 不得在 UI 看起来可点击时静默 `return`。
- 选择并实现一种明确策略：
  - per-item single-flight，允许不同项目并发且后端/Query key 能正确隔离；或
  - global serial，期间禁用所有相关开关并显示 `aria-busy`/可见保存状态；或
  - bounded queue，展示已排队状态。
- 成功仍以 backend authoritative reread 为准；失败恢复真实状态，不保留乐观成功。
- Agent Skills 与 Agent MCP 当前有高度重复的“mutate -> refetch -> verify -> feedback”流程。若复核后语义一致，提取一个 `shared/features` 的 authoritative-assignment controller/hook；domain-specific command/verification 通过 typed adapter 传入。
- 不把该 hook 泛化成所有异步操作的万能 mutation engine；至少两个真实 assignment consumer 和一致的回读/并发规则是共享前提。
- 共享 UI status/notice 可以复用，但 Skills/MCP 的错误文案、target ID 和 trust dialog 保持 domain-owned。

### 8. Shared component and module improvements

本任务必须评审并落地或明确拒绝以下公共 owner：

| Candidate | Expected owner | Reuse condition |
| --- | --- | --- |
| CSS-first selected treatment | `shared/ui` token/recipe | Navigation、Tabs、Catalog 至少两个语义一致消费者 |
| `FeatureTabs` Radix adapter | existing `shared/ui/FeatureTabs` | All exclusive page tabs use one wrapper |
| Authoritative assignment controller | `shared/features` | Skills and MCP share mutation/readback/concurrency semantics |
| Typed async action status surface | `shared/ui` | Install/Auth/assignment share visual states only; domain state machines remain separate |
| Route/domain draft boundary | owning page or `shared/features` | Only promote when two routes share lifecycle semantics, not merely `useState` |

- 实现中出现第二个真实消费者时，在同一任务内提升或记录明确 follow-up；不得等第三份重复。
- 共享组件必须保留最小 props、明确语义和独立测试；不得通过大量 boolean props 容纳不相关页面。
- 优先扩展现有 shared owner；新增包前检查 Radix、React、TanStack、现有 primitives 和维护成本。

### 9. Large-module architecture review

- 对 `ModelsPage`、Skills、MCP、Memory、Prompts、MCP catalog、Agent CSS 等大型模块进行职责图和依赖评审。
- 只在以下条件下拆分：
  - 有明确 props/domain contract；
  - 可以独立测试；
  - 可以独立 lazy-load；
  - 可以减少跨领域修改冲突；
  - 或当前文件同时拥有不应共存的 wire parsing、state machine、UI 和 CSS policy。
- 不以行数阈值单独要求拆分，也不创建 barrel/Context 来掩盖更高耦合。
- 优先拆分 Models 的 route orchestration、target panel registry、domain panel 和 apply state；MCP catalog 的静态 recipe/data 与编辑器逻辑也应分离评审。
- CSS 按 shared token/recipe 与 route-owned layout 分层；不为每个页面复制 selected/search/list/dialog recipe。

### 10. Performance and bundle budget

- 生产构建必须产生可识别的 route chunks；初始 Agent route 不下载 Models/Skills/MCP/Prompts/Memory 页面代码。
- 为 app-owned initial chunk 建立可维护 budget。目标是消除当前 >500 KB 单主 chunk warning；若 vendor chunk 仍大，记录来源和 reviewed budget，不通过提高 warning limit 隐藏问题。
- 测量 route 首次加载、route 切换、已访问多 route 后 DOM/observer/query 数量和交互响应。
- 避免用无证据的 `memo/useMemo` 大范围包裹；优化必须针对测量到的加载、render 或 observer 问题。

### 11. Test reliability and current-main UAT

- 修复 targeted V2 tests 中所有当前 React `act(...)` warning；禁止 stderr 过滤、console mock 吞警告或全局 suppression。
- 为关键 suites 增加 “unexpected React warning fails test” 的局部 guard，并区分框架已知 warning 与产品 warning；任何例外必须记录 upstream issue/version 和移除条件。
- Browser tests 增加：
  - Lens disabled/observer unavailable/delayed；
  - selected nav 点击右侧控件后仍清晰；
  - Tabs 完整 keyboard；
  - route lazy loading/hidden query；
  - tool buttons 无 noop；
  - assignment busy/queue behavior；
  - minimum window、760 boundary、900/1152/1232/1440 viewports、reduced motion。
- 重新评审 #141 的 A3-A8/B4/B7/B8 等前端 finding；逐项标记 `still applies | fixed | obsolete`，不继承历史版本结论。
- Prompt/Memory 数据破坏与 mixed-file Stage 0 blocker 不在本任务实现，但前端重构不得掩盖或回归其后续修复。
- Playwright/自动 accessibility 检查只能补充键盘、焦点、对比度和真实 WebView UAT，不能替代人工/原生验证。

### 12. Same-domain defect policy

测试期间发现与 V2 shell、selection、Tabs、route state、query lifecycle、assignment controls、responsive layout 或测试 warning 同域的问题，直接在本任务修复并补测试。涉及 backend data semantics、installer/Auth domain、发布签名或新产品功能的缺陷必须拆出并回链，不得扩大本任务。

## Non-goals

- 不推翻 V2 或回退到 leftover V1。
- 不更换整套设计系统，不引入 MUI/Ant Design 等第二套组件体系。
- 不为了状态保存增加全局 store。
- 不为了文件变短机械拆分所有大文件。
- 不删除所有动画/玻璃效果；语义状态可靠后可以保留装饰效果。
- 不实现新的 Search/Account 产品功能；只有已有真实范围时接线，否则移除/disabled。
- 不通过提高 Vite chunk warning limit、禁用 React warning 或减少测试断言来“通过”。
- 不处理 Prompt live file、Daily Memory 文件过滤、Windows installer 或 Auth backend 状态机本身。

## Acceptance Criteria

- [x] 前端 SPEC 不再要求 blanket keep-alive，也不把 Lens 作为唯一 selected-state owner。
- [x] SideNavigation/FeatureTabs/Catalog selected host 在 Lens 完全移除时仍有清晰、可访问、对比稳定的状态。
- [x] 点击右侧按钮、输入框、dialog 或 route content 后，左侧选中项不会变暗、丢失或定位到旧项。
- [x] `ResizeObserver`/MutationObserver 不可用或延迟、reduced-motion、hidden reveal 和 backdrop-filter fallback 均通过测试。
- [x] SelectionLens 不再递归观察整个 layout subtree；observer 数量与活动 host/track 有界。
- [x] `FeatureTabs` 内部使用 Radix Tabs，支持 Arrow/Home/End、roving focus 和 TabPanel ARIA；页面没有第二套 Tabs recipe。
- [x] Production 一级 routes 使用 lazy chunks；初始 route 不加载其余五个页面模块。
- [x] `PersistentPrimaryOutlet`、ModelsPage 和其他 route roots 不在 render 阶段 set state。
- [x] 非活动页面不继续自动 query/poll/observer；需要跨 route 的 backend job 不依赖隐藏 React tree 生存。
- [x] 未保存草稿、URL selection、backend resource 和 secret 的生命周期分别有明确 owner/测试。
- [x] Search/Settings/Account 不存在 focusable clickable noop；每个可见控件有真实结果。
- [x] Skills/MCP 忙碌期间没有静默吞点击；disabled/busy 语义和 authoritative reread 均可见、可测试。
- [x] Authoritative assignment 共享 owner 在 Skills/MCP 两个真实消费者中复用。
- [x] 大型模块拆分有职责/测试/lazy-load依据，不以行数作为唯一验收。
- [x] 生产构建消除 app main chunk >500 KB warning，形成可解释的 vendor/app/route budget；不提高 warning limit掩盖。
- [x] Targeted V2 unit/browser suites 无未经允许的 React warning；不使用 suppression。
- [x] #141 当前前端 finding 完成 latest-main 分类和证据回链。
- [ ] macOS Tauri WebView 与 Windows WebView2 installed-app UAT 验证 selected state、route switching、minimum window、keyboard、reduced motion 和长内容滚动。

## Dependencies

- Independent reliability slices can start immediately after review.
- Stage 1 and Stage 4 provide the final installation-target/Auth DTOs consumed by related Agent UI slices.

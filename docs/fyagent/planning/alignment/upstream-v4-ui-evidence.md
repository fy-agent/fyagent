# CC Switch v4.0.0：页面结构与渐进呈现的来源证据

审计日期：2026-10-04。证据级别：`code_audit`，辅以上游官方发布截图；没有把源码审计或发布截图当成 FyAgent 运行验收。

本报告只审计上游，不判断 FyAgent 当前实现。全部源码链接固定在 v4.0.0 对应提交 **`412579f9ad81ed3c045476198aa9f8d1de36a2ae`**。检查工作副本与该提交的差异，仅 README、合作方图片和发布说明存在后续修改；本报告涉及的组件、设计规范及发布截图与固定提交一致。正文不依赖工作副本 HEAD 的后续文案。

## 1. 可以直接指导复用的结论

v4 的复用对象应包括**页面组成、信息层级、状态计算与操作语义**，随后才是样式。最关键的约束是：

1. 供应商列表先回答“是哪一家、处于什么状态、剩余多少、现在能做什么”。模型全文、配置、诊断不同时铺开。
2. **模式标签切换查看内容，不会启用模式。** 查看与生效独立；只有明确的启用按钮及对应确认操作才改变模式。把分段标签直接接到配置写入，会破坏上游交互语义。
3. 聚合列表的行内操作是添加或移除成员，“设为默认”在更多菜单；默认供应商、已添加成员和可添加项分组，模型先显示数量。
4. 表单不是统一“所有内容藏进高级”。Claude/Codex 聚合简版聚焦连接与模型，少用参数折叠；Pi 的模型详情折叠，但配置 JSON 在选定配置后直接展示。必须按具体应用与状态复用。
5. 资源页复用“对象行 × 应用列”的矩阵；批量操作只在选择后出现，存储策略、恢复、诊断进入抽屉或菜单。
6. 会话正常结果优先，失败过程不能被整体折叠掩盖；用量先总量和趋势，数据维护与请求细节另开抽屉。

这些规则有规范和组件实现双重依据，不只是对截图的审美判断。下面逐页区分“代码事实”“规范明确说明”“审计推断”。

## 2. 专门设计资料确实存在，但最终代码优先

固定提交树中**没有 `.trellis/` 目录**；存在以下专门资料：

- [docs/design-system.html](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/docs/design-system.html#L716)：v7 设计系统，包含页面画板、组件和交互规约。716–717、1033–1038 明确规定每个视图一个实心主操作；重复操作用图标与可键盘触发的提示；状态留在对象原处；丢失、重启等后果写进动作或确认，而不是藏在帮助说明；按钮应说具体动作。
- [docs/pi-frontend-uiux-guidelines-zh.md](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/docs/pi-frontend-uiux-guidelines-zh.md#L15)：明确区分新手的短默认路径与熟练用户的完整控制，强调同类信息同一层级、复用家族组件、不制造影子状态；269–274 给出字段是否进入结构化界面的判断条件。
- [Pi 原生供应商同步要求](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/docs/pi-live-provider-sync-requirements-zh.md)、[Pi 原生契约](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/docs/pi-native-contract-zh.md)补充页面可以声称哪些状态。

资料不是逐条都与发布实现一致。已发现两项须避免直接搬用的旧规则：

- Pi 规范第 90 行写“新建直接展示完整基础表单”，但最终共享 `AddProviderDialog` 明确从 `pick` 进入 `form`，Pi 也使用该流程；见本报告表单段落。
- 设计系统第 1037 行还写有命令面板快捷键；固定版本的组件树和主入口未保留对应命令面板。不能仅凭该设计页在复用稿中恢复一个实际版本没有的入口。本报告不把“当前未挂载”扩大为已证明的历史删除原因。

官方截图用于交叉核对视觉组织：[供应商主页面](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/assets/release-notes/v4.0.0/main-zh.png)、[Apps](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/assets/release-notes/v4.0.0/apps-zh.png)、[聚合](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/assets/release-notes/v4.0.0/agg-zh.png)、[会话](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/assets/release-notes/v4.0.0/session-zh.png)、[用量](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/assets/release-notes/v4.0.0/usage-zh.png)。这些是上游发布资产；其中供应商名称、路径及数值属于示例上下文，不应原样成为产品演示数据。

## 3. Shell 与导航：应用上下文在上，全局资源在下

**默认显示。** 左侧应用导航承载各客户端；全局入口依次包括 MCP、Skills、Prompts、Sessions、授权中心、用量，底部保留 Apps 和 Settings。授权中心可带需重新授权提示，用量可带今日成本。进入设置后，原导航由设置目录替换，而不是继续在内容区叠一套侧栏。

**按需与折叠。** 侧栏可收窄；模式提示和需要注意的状态保留为紧凑图标/标记。设置目录有返回主导航动作，分通用、应用配置、路由、网络、数据、关于。

**避免呈现。** 不把每个应用下的 MCP/Skills 等复制成成套导航；不把所有设置分类展开在应用首页。代码所实现的是“当前应用对象 + 全局管理”的两层结构。

**主要动作与上下文。** 点击应用进入其供应商上下文；点击全局项进入相应资源页。授权异常和今日成本是导航层摘要，不替代页面内详细状态。

**证据。** `Sidebar`、`MainDirectory`、`SettingsDirectory`：[Sidebar.tsx 65–95](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/shell/Sidebar.tsx#L65)、[全局目录 228–247](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/shell/Sidebar.tsx#L228)、[底部入口 304–328](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/shell/Sidebar.tsx#L304)、[设置目录 604–675](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/shell/Sidebar.tsx#L604)。

**推断。** 这套结构减少同一资源在不同应用间来回寻找的成本；值得复用的是导航归属和状态密度，不能只复制侧栏颜色。

## 4. 供应商卡片：识别、状态、额度、动作四块

**默认显示。** 横向紧凑卡片以名称和少量状态标签为第一行；第二行放 URL、备注，或官方原生登录/托管账号信息；右侧固定额度区；末端为当前可执行的主动作、编辑与更多。是否显示健康状态由 `presentation` 决定。

**按需显示。** 编辑打开表单；更多菜单收纳复制、测试、用量配置、打开终端、删除，以及当前模式提供的菜单项。禁用项携带原因。官方来源不支持可靠探测时不会硬显示同样的测试操作。

**高级与避免。** 不在每张卡片里铺完整模型、全部连接参数和所有动作。账号已包含在名称中时避免副行重复；模型数量可作小标签，具体模型在编辑流程中处理。

**主要动作与上下文。** 卡片动作来自 `CardPresentation`，而不是所有应用共用一个无条件“切换”。实际可能是直连切换、路由目标切换、聚合添加/移除、原生成员启用/移除等。

**证据。** `ProviderCard`：[名称与身份区 375–478](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/providers/ProviderCard.tsx#L375)、[额度和动作区 481–568](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/providers/ProviderCard.tsx#L481)。`ProviderCardActions`：[104–224](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/providers/ProviderCardActions.tsx#L104)。规范对不展示模型列表的直接理由见 [Pi 规范 50–64](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/docs/pi-frontend-uiux-guidelines-zh.md#L50)：避免卡片高度失控和信息重复。

**推断。** 可整体复用卡片、操作菜单和展示状态模型；只复用外观、重新随意拼状态和按钮，会丢掉这层质量。

## 5. 直连／路由／聚合：查看和生效明确分离

**默认显示。** 进入页面落在实际生效模式；Claude/Codex 可有三种模式，Gemini CLI、Grok Build 隐藏聚合格。模式标签的选中背景表示“正在查看”，小状态点表示“正在生效”。只有查看生效模式时展示当前目标摘要；路由摘要只说目标，不把本机地址和端口塞到主行。

**真正启用。** 点击标签只调用 `onView(mode)`。查看非生效模式时，通知条同时说明正在查看和实际生效模式，并给“启用路由/聚合/回到直连”的具体按钮。进入路由或聚合打开 `ModeDialog`，选择目标并确认；回直连调用退出动作。离开再回或生效模式变化后，查看状态回到生效模式。

**聚合的默认结构。** 默认供应商、已添加成员、可添加项分组。成员卡先显示模型数；行内主操作只有添加/移除。“设为默认”在更多菜单：聚合已生效时会实际改变默认，直连状态只记下待选项，路由已生效时禁用并解释，因为默认与路由目标共用指针。

**按需与高级。** 故障转移开关和路由设置入口仅在查看且生效于路由时出现；路由设置使用抽屉。服务未运行、接管失败、Codex 客户端过期等通知由真实状态触发，并提供启动、回直连或重试，不能作为永久首页提示墙。

**避免呈现。** 不把标签点击当配置写入；不把三模式设计套到所有客户端；不在聚合成员行同时放添加、移除、默认、测试、删除五个主按钮；不把聚合称为额度合并或自动模型优化。

**证据。** `ModeTabs`：[89–124](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/providers/mode/ModeTabs.tsx#L89)。`SwitchModePanel`：[模式与双状态 71–108](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/providers/mode/SwitchModePanel.tsx#L71)、[状态摘要 268–302](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/providers/mode/SwitchModePanel.tsx#L268)、[启用动作与条件入口 456–514](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/providers/mode/SwitchModePanel.tsx#L456)、[确认与抽屉 538–572](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/providers/mode/SwitchModePanel.tsx#L538)。聚合行的语义见 `buildSwitchSections` 所在 [presentation.ts 438–515](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/providers/presentation.ts#L438)。

**规范明确／推断。** 源码 75–77 已明确“tab 只查看，生效须点写明后果的按钮”。据此推断，复用时必须保留操作与后果的绑定，不能为了减少一步而改变语义。

## 6. 添加与编辑：共享流程，应用及模式决定复杂度

**默认显示。** `AddProviderDialog` 是全屏面板，打开先选预设或自定义，再进表单；返回按钮在表单态回到选择页，在选择态回列表。表单仍挂载以保留共享状态，不另造一套聚合数据结构。

**聚合简版。** 实际处于聚合模式、应用为 Claude/Codex 且非官方供应商时可用简版。主要层级为认证/协议、Key、地址、模型；提供切回完整表单的动作。Claude 简版高级项主要是 User-Agent 与请求覆盖；Codex 还按协议放思考能力和 Anthropic 专属项。已有高级值会影响展开状态，不能把有值设置无声藏起来。简版不重复展示底部原始配置编辑器。

**按需显示。** 联网获取/选择模型、测速、认证类型专属字段随上下文出现。保存时如外部程序改了同一字段，才打开冲突对话框，列出冲突键，让用户选保留外部修改或覆盖；其余改动照常保存。失败继续保留编辑器。

**避免呈现。** 不把所有应用表单缩成同一组三字段；不把“有结构化表单”误写为“所有配置都隐藏”；不在首屏固定显示冲突选择或完整诊断。

**证据。** `AddProviderDialog`：[71–87](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/providers/AddProviderDialog.tsx#L71)、[527–550](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/providers/AddProviderDialog.tsx#L527)。`ProviderForm`：[674–680](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/providers/forms/ProviderForm.tsx#L674)、[881–887](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/providers/forms/ProviderForm.tsx#L881)、[2721](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/providers/forms/ProviderForm.tsx#L2721)。[ClaudeFormFields 922–980](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/providers/forms/ClaudeFormFields.tsx#L922)、[CodexFormFields 1401–1525](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/providers/forms/CodexFormFields.tsx#L1401)。`LiveEditConflictDialog` 与 `useLiveEditConflict`：[17–99](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/providers/LiveEditConflictDialog.tsx#L17)。

**推断。** 应优先复用共享表单和应用专属子组件，并接入目标产品已有保存契约；重新画一个“看起来简洁”的表单，容易损失有值展开、错误定位和冲突保存这些不在静态截图里的行为。

## 7. Pi 必须单列：管理成员，不接管当前模型

**默认显示。** 已加入与可加入的供应商；行内启用/移除，编辑和删除分离。卡片不显示模型列表，也不显示“当前供应商/当前模型/当前默认”。选择模型由 Pi 原生 `/model` 管理。

**上下文状态。** `models.json` 中显式存在供应商标识就是成员身份，不比较整份 JSON 制造漂移/所有权徽标。读取原生状态失败时，启用、移除、删除带禁用原因。全局默认指向将删除的供应商时可作非阻塞提醒，但页面不猜项目级 `.pi/settings.json` 上下文，也不改默认项。

**表单渐进呈现。** 模型行默认只放 ID、名称、展开、移除；每个模型的详情与思考级别映射初始都收起。展开后才出现推理、视觉、窗口、最大输出等能力；详情校验失败会展开对应区域。配置选择完成后，JSON 编辑器直接展示，不能把 Pi 强行套成 Claude/Codex 聚合简版的“完全隐藏原始配置”。

**避免呈现。** 不由第一模型推导默认，不由认证记录凭空生成已启用卡片，不把项目无关的全局列表声称为当前会话状态。Pi 旧文档中的“打开立即完整表单”已被共享添加流程覆盖，最终代码是预设选择→表单。

**证据。** [Pi UI 规范 50–80](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/docs/pi-frontend-uiux-guidelines-zh.md#L50)、[模型呈现规范 134–162](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/docs/pi-frontend-uiux-guidelines-zh.md#L134)。`buildAdditiveSections`：[presentation.ts 697–792](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/providers/presentation.ts#L697)。`PiProviderForm`：[初始折叠 563–568](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/providers/forms/PiProviderForm.tsx#L563)、[模型行 1653–1754](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/providers/forms/PiProviderForm.tsx#L1653)、[JSON 2104–2128](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/providers/forms/PiProviderForm.tsx#L2104)。

**规范明确。** 这不是视觉偏好，而是页面没有权威项目上下文，因此不能代表 Pi 作当前选择。复用时保留这一例外。

## 8. 额度：最多两行，临界值和恢复时间可扫描

**默认显示。** 卡片右侧最多两行、右对齐；多档额度时第一行固定最短窗口，其他档合并，过多时优先剩余最少的档。普通数值使用中性色，耗尽/失败才用危险状态；存在重置时间则额外留倒计时列。

**按需显示。** 悬停解释各档、更新时间和重置时间；有刷新能力时，悬停或键盘聚焦出现刷新图标，点击后转圈并短暂反馈成功/失败；刷新图标占左侧空白，数字位置不跳。查询失败有单独文案及原因，不伪装成正常余额。

**避免呈现。** 不把多套餐全部进度条铺在每张卡片，不用主题强调色让普通余额像按钮，不只在隐藏说明里放重置时间。也不能将不同提供方额度相加声称统一可用额度。

**证据。** `QuotaLines` 的行为注释及实现：[176–244](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/quota/QuotaLines.tsx#L176)、[256–320](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/quota/QuotaLines.tsx#L256)。`failedLines`、`pickLines`、`cardRows`：[quotaRules.ts 306–355](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/quota/quotaRules.ts#L306)。

**推断。** 复用价值是“有限空间内稳定比较”和状态反馈，不是单独画一个漂亮进度条。

## 9. Apps：安装事实与当前可行动作

**默认显示。** 表格式应用行展示名称、安装来源、可执行路径、当前版本、可用新版本、侧栏显示开关。页面头部是检查更新、条件式全部升级与最后检查时间。

**按需显示。** 未安装时给安装动作，有新版本时给升级；读取中或不可运行时不强行给同样操作。检测到多个安装才出现可展开提示，展开后看其他安装。升级有确认面板；诊断与手动命令在更多菜单，手动命令另开对话框。

**高级与避免。** 不在正常行铺诊断命令，不为已最新应用放无意义“升级”按钮；多个安装默认摘要而不是一口气展示全部路径。不能只用应用 Logo 卡片替代实际安装来源和版本。

**证据。** `AppsPage`：[初始折叠 108–109](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/apps/AppsPage.tsx#L108)、[头部 166–243](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/apps/AppsPage.tsx#L166)、[行状态与动作 432–513](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/apps/AppsPage.tsx#L432)、[其他安装条件展开 582](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/apps/AppsPage.tsx#L582)。

**推断。** 此页首先解决“本机到底装了什么、正在用哪份、能否更新”，首版直接复用这种信息架构比自行重画软件商店式首页更接近实际任务。

## 10. Skills：已安装矩阵优先，发现与维护分层

**默认显示。** `UnifiedSkillsPanel` 默认已安装；数量在页签，头部只在有更新时出现可点击的更新数量。行对应 Skill、列对应应用，辅以搜索、状态和来源筛选。“添加”是主动作，菜单分发现、ZIP、从应用导入。

**按需显示。** 发现页首次进入才挂载，可选择仓库或 skills.sh 来源；安装、重试、目录被占用等动作依对象状态呈现。选中 Skill 后，原筛选行替换为选择数量、按应用启用/停用、更新、卸载、取消；批量工具条不永久占首屏。

**高级与折叠。** 检查更新、恢复、仓库管理、存储管理在更多菜单；存储抽屉管理自动/链接/复制策略与路径，自动是初始策略。主列表没有把这些机制全部作为入门必选项。

**避免呈现。** 不让每行常显一组重复的全应用管理按钮；不把安装列表、市场和存储维护混成同一屏；不要把“链接/复制每次安装都必须先选”冒充上游默认设计。

**证据。** `UnifiedSkillsPanel`：[118–144](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/skills/UnifiedSkillsPanel.tsx#L118)、[头部与入口 882–981](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/skills/UnifiedSkillsPanel.tsx#L882)、[选择态 1096–1176](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/skills/UnifiedSkillsPanel.tsx#L1096)。`SkillsPage`：[发现页 519–658](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/skills/SkillsPage.tsx#L519)。`SkillsStorageSheet`：[35–71](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/skills/SkillsStorageSheet.tsx#L35)。

**推断。** 复用对象应包括 `PageTabs`、矩阵、选择态工具条与维护抽屉；不是只把现有卡片改成细边框。

## 11. MCP：矩阵表达部署状态，编辑器承接复杂配置

**默认显示。** 服务行优先 ID、传输类型和命令/地址摘要；应用列显示启用、未启用或失败。顶部添加是主动作，导入是次动作；搜索只覆盖安全的识别信息，源码特意排除凭证。

**按需显示。** 点击行或编辑图标打开抽屉。更多菜单放重新同步、复制配置等操作。应用写入失败按应用提示并可重试；矩阵单元格有独立失败状态，不能只保留全页成功提示。批量范围基于当前筛选可见对象。

**编辑器层级。** 默认表单页签，另有 JSON；可以从模板或粘贴内容开始。批量解析时展示解析结果、已存在项及跳过/覆盖选择。JSON 和敏感值有主动显示控制；名称等元数据扩展默认折叠。底部固定提交动作。

**避免呈现。** 不在列表显示密钥、完整 JSON 或所有元数据；不把部分应用失败压成一次全局成功；不让过滤后的批量动作悄悄作用于不可见对象。

**证据。** `UnifiedMcpPanel`：[搜索约束 50–69](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/mcp/UnifiedMcpPanel.tsx#L50)、[头部与失败提示 552–665](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/mcp/UnifiedMcpPanel.tsx#L552)、[行与矩阵 792–905](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/mcp/UnifiedMcpPanel.tsx#L792)。`McpFormModal`：[初始状态 138–160](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/mcp/McpFormModal.tsx#L138)、[模板/粘贴 651–711](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/mcp/McpFormModal.tsx#L651)、[元数据折叠 1214–1224](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/mcp/McpFormModal.tsx#L1214)。

**推断。** UI 复用要连同部分成功/失败、单元格重试一起迁移；只复用配置编辑表单仍会失去 v4 资源管理的主要信息层级。

## 12. Prompts：对象摘要留在列表，正文进入编辑抽屉

**默认显示。** 应用选择器、目标文件路径（适用时）、搜索；列表行显示名称、启用标记和截断的描述/详情。启用/停用是行内动作，编辑为图标，复制到其他应用、复制内容、删除在更多菜单。

**按需显示。** 点行进入 560 宽抽屉，编辑名称、描述、正文及字数/大小。保存按钮按实际后果命名：普通保存、保存并覆盖目标文件、Pi 的保存并写入。当前生效项的删除带阻止原因。正文不是列表里的展开大卡片。

**高级与避免。** 这里并非通过更多“高级折叠”取得简洁，而是列表与正文编辑职责分离。不要让每行同时展示整篇 Prompt；也不要把会写入原生文件的动作统一标成无后果的“保存”。

**证据。** `PromptPageFrame`：[96–179](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/prompts/PromptPageFrame.tsx#L96)。`PromptListItem`：[53–169](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/prompts/PromptListItem.tsx#L53)。`PromptFormPanel`：[143–178](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/prompts/PromptFormPanel.tsx#L143)、[正文与固定动作 188–329](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/prompts/PromptFormPanel.tsx#L188)。

**推断。** 可以直接复用页面框架、列表行、抽屉及具体动词，目标产品只需明确适配写入规则。

## 13. Sessions 列表：先找项目，再读会话

**默认显示。** 默认筛选传入应用，无有效应用时 Claude；默认按项目分组且项目收起，记忆用户展开的项目。常显应用筛选、搜索、时间/项目视图。行内优先标题、最后消息和相对时间。

**按需显示。** 行悬停/聚焦时，恢复与更多动作替换时间区；单应用筛选时省略重复应用图标，项目分组时省略每行目录。刷新、多选、终端设置、来源路径放更多菜单。进入选择模式才出现全选、选择计数和底部删除条；进入阅读页保留列表滚动位置。

**避免呈现。** 不把终端和来源路径维护放在每条会话的显著区域；不重复显示已由分组表达的信息；未选择或删除进行中禁用批量提交。

**证据。** `SessionManagerPage`：[默认与记忆 65–90](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/sessions/SessionManagerPage.tsx#L65)、[工具菜单 975–1012](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/sessions/SessionManagerPage.tsx#L975)、[列表与筛选 1050–1195](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/sessions/SessionManagerPage.tsx#L1050)、[批量态 1240–1375](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/sessions/SessionManagerPage.tsx#L1240)。`SessionItem`：[172–249](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/sessions/SessionItem.tsx#L172)。

**推断。** 首屏优化对象是“找到继续工作的会话”，不是展示最多会话字段。

## 14. Session Reader：回复优先，失败过程不能隐身

**默认显示。** 默认“全部”视图；注入内容、查找、导出思考内容关闭；展开全部过程偏好默认否，目录默认开并记忆。正常有最终回复的执行过程收起；中断、无最终回复或步骤搜索命中时自动展开。

**按需与异常。** 收起过程仍可露出最多 5 个失败步骤，失败详情自动显示 6 行；普通展开预览 12 行。成功参数通常折叠，失败、无结果或 shell 参数截断时展开；输出点“全部”再取全文，过大给源路径。目录按提问/回复跳转并跟随当前轮，窄窗隐藏。

**主要动作。** 恢复具备命令和终端时直接执行，仅有命令时变成复制，无命令时禁用并给原因。复制/导出 Markdown、附带思考、ID、源路径、重读、删除放更多菜单。全部/对话/改动三选一；展开过程与注入内容在视图菜单。

**避免呈现。** 不默认倾倒所有工具输入输出；也不以“过程已折叠”为由隐藏失败。结构化阅读不证明跨平台恢复能力，恢复动作仍必须依据可执行命令和终端条件。

**证据。** `SessionReader`：[177–194](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/sessions/reader/SessionReader.tsx#L177)。`isTimelineExpanded` 等规则：[turns.ts 780–908](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/sessions/reader/turns.ts#L780)。`SessionReaderHeader`：[136–258](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/sessions/reader/SessionReaderHeader.tsx#L136)。`SessionStepDetails`：[347–445](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/sessions/reader/SessionStepDetails.tsx#L347)。`SessionOutline`：[42–96](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/sessions/reader/SessionOutline.tsx#L42)。

**推断。** 值得整体复用的是轮次结构、失败例外、按需载入与上下文恢复逻辑；仅把消息泡泡重画一遍不足以对齐。

## 15. Usage：先总量与趋势，后请求与维护

**默认显示。** 今天、全部应用、请求日志；页面顺序为筛选→成本/请求数/真实 Tokens/缓存命中率四项→趋势→日志/供应商/模型/定价。全部日期用热力图，其余用趋势图。应用变更清供应商和模型，供应商变更清模型，避免遗留无结果组合。

**按需显示。** 更多指标才展示新增输入、输出、缓存写入/读取；未上报的缓存写入标 N/A。点击日志行打开请求详情抽屉，按基本信息、Tokens、费用、性能分块；请求模型/计价模型只有与实际模型不同才显示，倍率非 1 才显示，错误有内容才显示。

**维护与异常。** 同步是主操作，同步中禁用；全库无请求时不画全零总览，而给同步及配置定价。自动扫描、覆盖范围、路由记录设置、Codex 重建放数据来源抽屉；重建另确认。会话来源不展示不存在的首字计时，速度/耗时标估算或无计时；加载失败可重试，与记录不存在分开。

**避免呈现。** 请求详情内部不是高级折叠，它本身已是第二层；没有请求/响应正文展示。不要把零值当未知，也不要把不同采集来源的估算表现为同等精确。

**证据。** `UsageDashboard`：[初始与筛选 159–215](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/usage/UsageDashboard.tsx#L159)、[页面层级与空态 670–801](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/usage/UsageDashboard.tsx#L670)。`UsageHero`：[148–162](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/usage/UsageHero.tsx#L148)、[215–301](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/usage/UsageHero.tsx#L215)。`UsageDataSourcesSheet`：[118–200](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/usage/UsageDataSourcesSheet.tsx#L118)。`RequestDetailBody`：[RequestDetailPanel.tsx 89–407](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/usage/RequestDetailPanel.tsx#L89)。

**推断。** “减少字段”必须保留来源和计量差异；否则首版看似整齐，实则降低数据可信度。

## 16. Settings 与授权：常规设置分区，账号问题原处处理

**设置默认。** 专用设置导航分区；路径类设置手动保存，部分普通设置自动保存，失败有回退逻辑。网络、路由、数据维护各在自己的页面，不堆在供应商列表。

**设置按需。** 数据页容纳导入、备份、WebDAV；需重启的修改出现对应提示。路由页的服务、故障转移、修正器是常显分块，不应将“迁入专用设置页”错误描述为“全部默认折叠”。退出全部接管另确认。

**授权默认。** Copilot、ChatGPT、xAI 三组账号卡片；侧栏授权中心与供应商表单的管理账号入口复用同一内容。已有页头说明时不再重复一段介绍；从表单跳来会滚到对应服务。

**授权按需。** 空组给登录，已有账号给添加；账号行展示默认、使用它的供应商和适用额度。需重新授权时明确给重新授权动作并不显示正常额度；设为默认、重新授权、删除等放条件菜单。设备码和打开浏览器动作仅在相应登录流程中出现。

**避免呈现。** 不默认露出令牌正文，不把未知/过期授权画成正常额度；不在供应商页重复完整账号管理表；不因为叫“设置”就把所有分块做成无法扫描的多层折叠。

**证据。** `SettingsPage`：[保存语义 143–188](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/settings/SettingsPage.tsx#L143)、[分区 226–370](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/settings/SettingsPage.tsx#L226)。`RoutingSection`：[353–383](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/settings/sections/RoutingSection.tsx#L353)。`AuthCenterPanel`：[8–68](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/settings/AuthCenterPanel.tsx#L8)。`ManagedAccountsGroup`：[217–242](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/settings/auth/ManagedAccountsGroup.tsx#L217)、[空态与登录 329–371](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/settings/auth/ManagedAccountsGroup.tsx#L329)、[账号行 508–629](https://github.com/farion1231/cc-switch/blob/412579f9ad81ed3c045476198aa9f8d1de36a2ae/src/components/settings/auth/ManagedAccountsGroup.tsx#L508)。

## 17. 对复用工作包的直接约束与未验收项

建议把上游代码按页面簇接入并保留原交互骨架：

| 可成组复用的源码 | 必须一起保留的语义 | 不能只搬的部分 |
|---|---|---|
| Sidebar、PageHeader、PageTabs、Sheet、HoverTip | 导航归属、返回关系、一个主操作、键盘可达 | 只有间距和颜色 |
| CardPresentation、ProviderCard、ProviderCardActions、ModeTabs/Panel | 查看与生效分离、按上下文给动作、禁用原因 | 外观相同但所有卡都叫“切换” |
| ProviderForm 与各应用子表单、LiveEditConflictDialog | 有值展开、模式条件、保存冲突、未知字段保留 | 重新设计简化表单后丢失高级值 |
| Skills/MCP 矩阵与维护抽屉 | 每应用状态、部分失败、过滤范围、选择态批量动作 | 只有资源卡片和安装按钮 |
| PromptPageFrame/ListItem/FormPanel | 摘要与正文分层、写入后果文案 | 所有保存统一一个词 |
| Sessions Reader 与 turns/step 规则 | 正常折叠、失败例外、按需输出、真实恢复条件 | 只有消息排版 |
| UsageHero/Dashboard/Sources/RequestDetail | 来源差异、估算标识、空态、二层详情 | 只有四个指标和图表 |

本报告没有建议直接替换目标产品的数据写入权威、数据库迁移或客户端所有权契约。**复用上游 UI 和代码，不等于可以绕开目标产品已有变更预览、补丁写入与恢复机制。** 这些接入边界须由对应实现审计确定；本报告未读取 FyAgent 源码以避免重复审计。

本轮已覆盖 14 个主要界面/子界面：Shell、供应商卡、模式/聚合、编辑表单、Pi、额度、Apps、Skills、MCP、Prompts、会话列表、Reader、Usage/请求详情、Settings/Auth。尚未执行上游应用的登录、升级、真实配置写入、跨平台恢复或真实账号额度验证；未声称逐像素复刻、交互性能或所有平台布局已通过。发布图只能验证其截图中的构图，源代码只能证明对应实现与条件；正式移植验收应另保留运行截图和交互结果。

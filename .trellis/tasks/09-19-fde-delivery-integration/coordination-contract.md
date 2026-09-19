# 总控接口与范围决定 v0

2026-09-19。依据四线已出现的研究证据先冻结不依赖具体实现的边界；细节DTO由下轮与主控收敛。

1. 项目是新FDE能力的上下文身份，采用独立领域与持久化，不调用旧 apply_profile。保留旧Profile和所有现有全局配置。选择项目本身只改变可见上下文，绝不隐式改全局文件。
2. 客户项目能力本轮需要UI+原生CRUD+revision约束+资源绑定+受控项目文件产出，并验证A/B不会串写。仅原生metadata CRUD或只加下拉框不满足最终验收。至少一种已确认可隔离的CLI配置/工作目录适配必须形成可操作路径；不支持的Agent明确不可用。若无法可靠启动，可先输出明确的项目工作目录与配置产物及有证据的启动说明，但不能宣称已运行。主控验收将区分 materialized 与 launched。
3. 隔离承诺限于FyAgent管理的项目数据、配置与工作目录边界；不宣称普通用户进程获得操作系统级文件沙箱。外部Agent执行器仍承担其自身权限机制。
4. 资源属于project revision固定的引用/快照；秘密只在native owner解析，renderer不取得SecretRef或material。可分享kit仅含凭据需求槽位，不含设备引用、真实账号、客户数据、任意脚本或自动启用标志。
5. 交付包拥有manifest、内置目录、parse/validate/preview/import/export、包侧样例与纯检查；项目线拥有项目绑定事务，验证线拥有长期验证记录。各线只经窄typed接口调用，不直写其他域表。
6. 验证阶段独立：saved/auth/tool/sample/customer；每个事实必须带scope、来源、时间、revision/fingerprint与结果。当前配置变化后旧证据不得继续作为current pass。机器检查绝不产生customer accepted。
7. 首个完整演示为经营周报：授权/受控本地样本→包绑定→输入/指标校验→有来源结果→交接导出。知识客服与经营查询提供真实可用的准备包及各自失败样例；没有真实服务授权时不装成线上通过。
8. 主控最终接线：主导航/路由/懒加载；Rust composition/command registration/ACL；统一schema版本及sync保留策略；跨线feature ports/facade。各线可以在自己的分支添加必需接线并完整验证，但交付时列出接线文件和最小patch，主控逐项裁决合并，禁止自主改其他工作树。
9. 新增项目与证据表包含本地目录/身份引用时默认local-only；迁移必须fresh+old+rollback+sync对称测试，不能暗改现有表数据。
10. 本次所有实现以GPT-6 Astra / fast执行；每线先针对性检查，root统一跑最终宽检查。依赖/缓存写入属于构建授权，主控下一轮恢复为当前任务原有full-access环境，但不得触碰本机真实账号/Agent配置或安装应用。

## 可以先实现的独立工作

配置可靠性：Health只读观察、MCP新增默认最小分配、导入保留结构、readiness解释、发布staple顺序、Vitest安全升级等，在主控核验该线方案后即可启动，无需等待项目/包/验证架构全部完成。

## v1：2026-09-19 主控已批准实施

- 四线全部直接进入实现，无需再等规划或用户批准。各线在自己的工作树提交可审查结果，主控集成。
- 配置线：批准新建 pure observe_overview 供 Health 使用，保留既有管理页 reconcile；新增 MCP 默认零分配，明确上下文仅选当前 Agent；漂移只读解释；configurationEligibility 与安装/升级权限分开；依赖只做经过兼容验证的升级，glib/rand 不盲改锁文件。
- 数据库：两条数据线各自使用 schema 22 编译与测试；交付时把迁移逻辑封装为各自领域函数，root 最终合成唯一 21→22 additive migration，不能重复抢同名迁移函数。新增表均 local-only，并成对加入 sync skip/preserve。root 负责最终版本和冲突裁决。
- 项目身份：camelCase public DTO，projectId 为 opaque UUID，projectRevision 为 native 安全非负整数；mutation 带 expectedRevision。projects owner 对外提供只读 native ProjectDependencySnapshot（id、revision、状态、受绑定包 id/version/digest、无秘密资源摘要与可信 credential generation/不可验证状态），evidence 不查询/修改项目表。
- 包：批准 .fyagent-kit.json、纯文本 v1、native immutable library、主项目页入口、导入不启用、不带凭据引用；项目绑定由 projects owner 提供 bindDeliveryKit(projectId, expectedRevision, kitId, kitVersion, manifestDigest, bindingIntentId) 原子且幂等。projects 不复制 manifest，kit 不写项目表。可先独立完成包库/校验器/UI，再由 root 接绑定。
- 验证：统一使用 projectId/projectRevision，包可选 kitId/kitVersion/manifestDigest；native 依赖快照 reader 接口是权威，renderer 不能提供判定/指纹。尚无其他领域时以不可用的 adapter 明确失败，不使用生产 fixture/mock 成功。配置回读分 DB 与实际投影；saved_model_probe 必须绑定保存对象和请求前后依赖。
- 秘密版本：优先使用 secret owner 现有不透明版本；若没有，采取 native 私有持久代际/HMAC（key 不进入导出、日志、同步），或明确 unverifiable。绝不裸哈希秘密。模型精确名默认严格匹配；别名只能来自可确认映射，无法确认返回 unknown，不能把 HTTP 200 升级认证/调用通过。
- 实时结果 TTL：authentication_available/tool_callable 最长 15 分钟，显式 expiresAt；配置和样本依赖修订失效，无无限在线承诺；客户人工登记没有机器 TTL，但其依据失效会传递。未来时间和时钟回退 unverifiable。
- MCP 首期采用 A：不新增任意 stdio 命令执行器；真实未检验工具显示未检查，可登记带出处的外部人工记录。纯本机包 validator 使用 sourceClass=local_fixture，不证明线上 MCP/客户已验收。
- 人工客户验收：要求验收人、角色、范围、发生时间和可核对依据；可对真实外部验收登记为 manual_record，即便无本机机器证据也如实标注人工来源；仅 fixture/无依据不得自动或人工冒充真实客户验收。
- UI：项目线拥有唯一新的一级 Projects 页面（沿用现有 UI 词汇），包线与验证线各导出 projectId/context props 的独立面板，root 组合页签并接路由。可在分支作最小可撤回接线完成测试，但不要各加一个顶级入口。
- 项目运行适配：本轮先做已批准 S1 及受控产出，同时尽早启动一项 CLI 的局部实验。证据足够时完成可用的隔离配置/工作目录路径；不必等待其他功能。不能承诺 OS 沙箱或以 metadata-only 冒充完整功能。
- 交付：implementation-status.json 每个阶段落盘（状态/已完成/当前检查/真实阻塞），最终 implementation-ready.json + 变更/测试/接口/已知限制报告，分段提交到本线 codex 分支。不要发布/合 main/修改原共享 checkout/安装应用。

## v2：用户范围要求与独立规划审查

用户再次明确四线都用快速模式，阶段完成后由独立子 Agent 审查。更新聚焦 FDE 特定强化与补全，不扩成通用运行平台；前端只呈现任务、结果、下一步和真实限制，不展示工程注释、接线状态、内部 ID 或审计术语。必要业务说明保留，内部实现注释放代码和内部文档。

独立规划审查仅提出一项必须补齐，root 认领：包检查至长期证据的生产桥接。

- 用户从项目中执行已绑定包的闭合本机检查；请求只携带 projectId、expectedRevision、闭合 checker/fixture 标识和 runId，不能携带 outcome、指纹、任意命令或路径。
- evidence 执行服务通过项目 owner 读取依赖 A，确认确切绑定包，再调用 kits owner 的真实封闭检查器；完成后读取依赖 B。A/B 匹配且判定完成才追加 local_fixture/sample_passed 记录，变化或无法确认则 stale/unverifiable。
- 包结果、来源、输入和检查器版本以安全 DTO 保存；不持久化原始秘密或真实客户输入。写后回读，再由项目页刷新呈现。
- 不能把测试注入适配器放到生产，不能由前端把“通过”提交入库。root 负责在统一 composition 接入两个 owner，各线提供窄接口和实际模块后即可桥接。
- 一条真实原生验收贯穿 A 项目绑定周报包→运行成功和失败样例→刷新仍能看到→导出准确交接；B 项目无记录，改 A 修订后旧结果失效。

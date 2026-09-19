# FDE 规划阶段独立审查

审查日期：2026-09-19。范围：主控协调合同 v1、阶段审查政策，以及 reliability / projects / kits / evidence 四线的 PRD、design、implement、planning-ready。采用各 design/implement 中明确取代旧规划的批准合同，不将旧 planning-ready 中的未批准状态误报为当前阻塞。

本次只审查规划，不读取或修改正在实施的产品代码，不执行产品测试，不据此声称实现、原生运行或客户验收通过。

## 结论

**可继续并行实现；集成验收前必须补齐 1 个跨线执行合同。** 创建客户项目、包的不可变版本与原子绑定、依赖失效、交接导出的权威与资料边界已合理划分。未发现应停止独立编码的架构冲突，也未发现为了 FDE 另造通用编排、凭据库或脚本执行平台的要求。

## 必须修：明确经营周报检查的生产 native 入口及证据落盘闭环

**归属：root 负责冻结，kits 提供真实纯检查器，evidence 负责执行编排与记录；projects 继续只提供绑定依赖快照。**

证据位置：

- 主控 `coordination-contract.md:11` 要求“包绑定 → 输入/指标校验 → 有来源结果 → 交接导出”；`:27` 禁止 renderer 提供机器判定，`:30` 允许真实本机包 validator 产出 `local_fixture` 来源。
- kits `design.md:57-60` 将包检查之后的证据持久化列为实际数据流；`:73-79` 的接收仍是候选事实 DTO，`:77` 的本线 native port 只有独立的 `runBuiltinDemo`，没有冻结与项目绑定及长期记录的调用关系。kits `prd.md:34` 明确要求正反样例结果交 evidence owner 并回读。
- evidence `design.md:62-64` 的闭合 checker registry 目前只明确配置回读、模型探测及“测试注入”的样本 checker，生产 IPC 不接受伪造 fixture 来源；`:96-98` 的批准合同仍未指定可在生产中消费 kits 实际检查器的入口。evidence `implement.md:14` 的执行切片也只点名配置回读和模型探测。

**影响：** 若按各线现有独立清单完成，包页能够显示本次合成周报结果，验证页却仍可能没有可持久化的样本检查入口。此时刷新或导出交接后仍为“未检查”，或为了接通而错误接受前端传入的 `passed`。这直接截断首个完整 FDE 演示；“生产不得注入测试成功”本身不能替代真实内置样本检查的集成。

**最小修正：** 冻结一个明确的 native 路径即可，无需新增通用事件或检查框架。建议 evidence 现有执行服务接收 projectId、expectedRevision 与 checkId，从 projects 的权威快照取得已绑定 kit 身份，由 kits 的 native 函数实际执行指定不可变包内的封闭检查器；检查前后重读依赖，在 evidence 单事务中记录输入摘要、validator 版本、真实结果、来源类别和安全的来源行/数值摘要，再回读给两个面板和交接导出。前端只提交动作与身份，不提交机器 outcome 或指纹。未绑定、版本变化、库缺失及落盘失败分别准确返回，不能显示为已记录。

**最小验收：** 在一个临时原生环境完成“建 A 项目 → 绑定周报包 → 执行真实内置检查 → 刷新 → 导出”，导出能回读到同一条样本记录及其合成来源；B 项目不出现该记录。复用已有负例补一项检查中换包/修订或记录失败，确认不保留当前通过。无需为此重复全库测试。

## 建议

无其他必须新增的建议。本轮不对模板、旧阶段措辞或尚在实现中的接口命名作形式性挑刺。

## 已核对的非问题

- 项目不调用旧 Profile apply；选择项目不隐式修改全局配置。项目受管文件和 CLI materialization/launch 已明确分层，失败不升级为隔离运行成功（projects design:42-56、66-70；主控合同:6、33）。
- 包只有凭据需求槽位；本机项目凭据绑定与 secret generation 留在 native；包分享和交接导出分别排除客户原始输入及秘密引用，不存在互相要求泄漏 SecretRef 的合同（kits design:81-87；evidence design:45-47、80）。
- 项目绑定、包内容、证据记录各有唯一 owner；迁移统一合成且 local-only sync skip/preserve 成对处理（主控合同:24-27）。
- Health 保持只读观察，MCP 默认不分配；真实工具调用和客户验收不由配置成功或内置样本自动推导（reliability design:15-19、29-31、55；evidence design:13-27、96）。
- UI 为一个 Projects 一级入口及两个业务面板。项目修订/摘要等内部值属于接口与导出取证，不应作为普通产品解释文案；阶段政策已经明确约束，无需再加一套前端“审计”流程。

路径约定：本报告中的四线相对位置分别位于 `~/.codex/worktrees/fyagent-fde-{reliability,projects,kits,evidence}/fyagent/.trellis/tasks/09-19-fde-{config-reliability,project-isolation,delivery-kits,verification-handoff}/`；主控文件位于本报告上级任务目录。

# 验证与交接线独立阶段审查

固定审查：`861e1d6364ae3a70ef85bb978ea14872a9efeb1c` + `08ed44660a0d9fd92ab53d566c01e7947ac6d173`，相对 `fd1c8446`。只读审查源码、交付测试与本线 spec；没有修改产品、没有重复运行全库。root 正在接真实 project/kit reader，不把未接 adapter 另报缺陷。

结论：发现 4 项实质问题，需要收口后再做最终 FDE 路径验收。

## 1. P1：后续模型失败后，旧通过仍是 current，并可继续作为客户验收依据

位置：`src-tauri/src/services/verification/domain.rs:235-278`；关联 `mod.rs:450-489`、`VerificationPanel.tsx:402-406`。

复现场景：同一项目/模型/本地依赖在第一次检查返回通过，服务端撤销凭据或不可用，第二次检查返回 401。第二条 Failed 成功持久化，但第一条 Passed 在 TTL 内仍满足依赖与时间判断，继续得到 Current。页面会同时显示“失败”和旧“通过”，旧条目仍进入人工登记的可选依据，导出仍把它标为当前有效。取消也仅追加 Cancelled；前端 needsReview 只覆盖 IPC 抛错，刷新后恢复，不是持久的失效处理。

最小修复：在 native validity 投影中按同 checker、fixture 和依赖范围处理后续检查；后续非通过使旧通过待复核，历史仍保留，并让依据传播自然失效。不要把 missing_field 反例和 baseline 视为同一检查，避免合法反例使正常样例失效。增加“先通过→同范围失败/取消→刷新/导出/人工依据”的定向用例；错误直接退出未能持久化时，不可让仅 renderer 的临时提示充当永久事实。

## 2. P2：样例记录未保存业务数值，UI/Markdown 也丢失具体失败原因

位置：`src-tauri/src/services/verification/types.rs:59-63`，`export.rs:78-91`，`src/shared/features/verification/VerificationPanel.tsx:244-245`。

SampleReceipt 只有 inputDigest、code 和 matchesExpectation。包 validator 实际有 currentMinor、previousMinor、growthBps、targetBps 与 sourceRowIds，但 receipt 接口无法携带这些数据。因此完成周报样例后，持久记录和交接包无法回答本期/上期金额、增长率和目标完成率，刷新后只剩“业务输入通过”。失败 code 虽进入 JSON，UI 与 Markdown 将所有非 ok 原因压成“未通过”，交接人看不到是缺字段、重复行还是口径不一致。

最小修复：将 native validator 的固定结构业务指标保存到 receipt（可空，非原始输入文档），同步严格 DTO、持久读取、UI 和两种导出；给 closed SampleCode 提供简洁业务原因映射。root bridge 直接传已验证的结果，不能由前端填写数值。加正常周报数值刷新/导出一致和失败原因保留测试。

## 3. P2：常见文档链接无法登记为可追溯的外部验收依据

位置：`src-tauri/src/services/verification/domain.rs:6-18,142-145`；`src/domain/verification/index.ts:52-65`；`VerificationPanel.tsx:497-505`。

externalBasis.reference 复用 safe_label；允许字符没有 `:`、`/`、`.`，所以常见 `https://example.feishu.cn/docx/...`、共享文档链接均在前后端被拒绝。UI 只提供“外部真实记录编号”，没有另一个可保存文档地址的字段。FDE 常用文档作为交接/签收依据时只能写一个脱离地址的标签，不能直接定位记录。本线 spec 把依据自主收窄为 label，此收窄不能满足本次明确要核对的文档链接使用路径。

最小修复：给 reference 独立的有界类型，支持已有记录编号和不含 userinfo/凭据参数的 HTTPS 文档地址；继续拒绝本机路径、脚本协议、控制字符及秘密。不要放宽所有人工字段，也不要自动联网读取链接。UI 标注“记录编号或文档链接”，导出保留用户明确登记的依据地址，做正常飞书链接与含凭据链接的定向校验。

## 4. P2：带 reasoning effort 后缀的模型被永久误判为身份不确认

位置：`src-tauri/src/services/model_probe.rs:66,120-127`；已有请求投影 `:286` 与 `:417-425`。

build_probe_spec 会将 `gpt-x@high` / `gpt-x#high` 解析成请求模型 `gpt-x` 与 reasoning effort。但新 probe_saved_identity 丢弃 build_probe_spec 返回的 actual_model，拿响应 model 去匹配原始 `gpt-x@high`。即使服务准确响应请求模型 `gpt-x` 并返回内容，也必定 Unconfirmed，FDE 无法获得正确的模型检查结果。

最小修复：保留 request builder 返回的 actual_model，严格对照实际发送的模型 ID。不要推断别名。增加 effort 后缀命中、实际返回其他模型仍拒绝的 loopback 用例即可。

## 已核对的正确边界

- native 前后读取依赖；变化标 invalidated_during_run，项目/kit/资源/凭据代际变化、TTL、未来时间和依据撤销可传播失效。A→B→A 的单调代际真实性依赖 root reader，未重复判缺陷。
- runId 从 renderer 传入，但持久化重放核对 project/revision/checker/fixture；冲突拒绝。样例选择闭集，真实结果来自 native receipt，前端没有提交 machine outcome 的入口。
- customer_accepted 只能人工来源；要求人、角色、范围、时间与外部/真实依据；fixture 经人工中转仍不能变成客户验收。
- 对外 DTO 排除 credential generation、resource revision、SecretRef、原始响应与配置；导出重新读取状态并使用原生保存对话框。人工登记的姓名/范围等明确包含在预览里。
- 严格模型检查不以 HTTP 200 单独判通过，检查身份+输出、限制体积/超时，并处理显式失败事件。问题 4 是请求标准化与核对目标不一致，不要求放松身份检查。
- 本审查不把定向 fixture 测试描述为真实客户服务或本机 UI 验收。

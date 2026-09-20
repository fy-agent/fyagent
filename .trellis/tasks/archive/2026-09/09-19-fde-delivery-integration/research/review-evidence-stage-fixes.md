# 验证与交接阶段修复交付

按固定阶段报告 `evidence-stage-861e1d63-08ed4466.md` 的四项实质问题完成最小修复；未提交。没有修改 root 的 fde_workspace bridge、schema/backup/store/lib 或其他 domain。原生代码已通知 root 冻结并进入组合检查。

## 修复

1. 后续同 stage/checker/fixture/依赖快照的非通过，使更早 Passed 在 native 投影中变为 Stale。历史不删除、不同样例独立；人工依据验证与导出复用该投影。使用已有 validity memo 预填失效结果，撤销仍优先，不添加新状态表或运行框架。
2. externalBasis.reference 保留原受限标签，并支持有界 HTTPS 文档链接。拒绝 userinfo、查询/fragment、编码/控制字符、脚本/本机路径及常见秘密标记。人物/角色/范围仍使用原标签约束；不联网读取链接。前端提示改为“记录编号或文档链接”。
3. 身份检查严格对照 build_probe_spec 实际发送的 model ID，正确处理 @/# reasoning effort 后缀；不推断未知模型别名。
4. SampleReceipt 增加 native 业务数值、来源行与 validator；严格读取和两种导出均保留，UI 展示本期/上期金额、增长与目标完成比例。失败显示具体业务原因，metrics 为 None。root 将实际 kit validator 输出映射到这些字段。

## SampleReceipt 最终接口

- 既有 `input_digest`、`code`、`matches_expectation` 保留。
- `metrics: Option<SampleMetrics>`：`current_minor / previous_minor / growth_bps / target_bps`，camelCase DTO，限定 JS safe integer。
- `source_row_ids: Vec<String>`：闭合 builtin `row-1` 到 `row-6`，最多 6 个且不重复。
- `validator: String`：当前固定 `weekly-report/v1`。
- 使用 Evidence 既有 observed_at，不增加重复 checked_at。
- Ok 必须有 metrics；失败必须无 metrics。原始输入、配置、凭据及 native 依赖代际不导出。

## 验证

最终变更后：

- `mise run test:unit -- tests/renderer/features/verification.test.ts tests/renderer/features/VerificationPanel.test.tsx tests/renderer/platform/verificationPort.test.ts`：3 个文件，13/13 通过。
- `mise run typecheck`：通过。
- 改动的 4 个 TS/TSX 文件定向 ESLint：通过。
- 指定文件格式化、diff check：通过。先前控制字符 regex lint 问题已改成逐字符校验后通过。
- 新增原生定向测试：后续 Failed/Unknown/Unsupported/Cancelled 失效和人工依据拒绝；正常文档链接与秘密/脚本/路径拒绝；样例业务数值在重启/导出保持且负例互不覆盖；effort 后缀精确模型匹配。
- **原生测试未由本 agent 重跑**，按主控要求交给统一组合测试/Clippy。不得把前端 fixture 通过当作真实服务或原生 UI 验收。

## 文件清单

- src-tauri/src/services/verification/{types,domain,mod,export,tests}.rs
- src-tauri/src/services/model_probe.rs
- src/domain/verification/index.ts
- src/shared/features/verification/VerificationPanel.tsx
- tests/renderer/features/verification.test.ts
- tests/renderer/features/VerificationPanel.test.tsx
- .trellis/spec/backend/project-verification.md
- .trellis/spec/frontend/project-verification.md

保持本轮范围：不扩展任意样本执行、远端客户签署验证或通用 URL 抓取。root 继续独立复核真实桥接、本机交付流程和统一原生检查。

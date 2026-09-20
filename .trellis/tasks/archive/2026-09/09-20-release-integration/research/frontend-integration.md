# FDE 与订阅前端集成

更新：2026-09-20。范围为 FDE `6f38c213` 合入订阅分支后的 Models 前端冲突与对应回归。本次实现已冻结；没有 stage、commit、push，没有改原生代码、根控拥有的 ACL/route-chunk 合约或其他执行者的修改。

## 修复与保留的行为

1. `src/pages/models/OpenCodeModelsPanel.tsx`：解决文本冲突。原始快照继续判断是否存在托管投影；普通 API 编辑的供应商列表先排除保留的托管 ID，再沿用 FDE 的精确供应商选择、优先选择可编辑供应商、内置供应商只读展示与新建入口。普通保存携带所选 `providerId`；不通过显示名称重新定位供应商。
2. 面板的保存、覆盖确认和删除入口同时检查 FDE 的只读限制及订阅的托管/未知状态限制；切换供应商不会清除 Models 父级写入阻断。保留同一写锁、订阅确认时捕获的 OpenCode revision、双 owner 回读和恢复完成后的 picker 重挂。供应商切换确认打开时，订阅绑定和恢复入口暂停，避免两个确认流程交叠。
3. `src/shared/platform/tauri/features.ts`：修复无文本冲突的语义遗漏。FDE 把 Models Port 改为延迟加载时，自动合并遗漏了 OpenCode 的 `bindManagedProxy` 与 `restoreManagedProxy` façade；现已补齐。保留 `features → models → managedSubscriptions` 两级延迟加载，未改成 eager import，也未放宽 chunk 预算。
4. 合并后的 provider `filter/find` 使 React Compiler 无法保持已有手工 `useMemo(modelIds)`；现将这一个纯过滤表达式直接派生，数据流和过滤语义不变。首次 lint 失败保留在 `integration-frontend-lint.log`，修复后的最终 lint 通过。

已逐项核对自动合并文件，无需重复改动：`src/domain/configuration/types.ts` 保留 FDE 内置 provider 的可选 `npm` 与额外配置字段；`src/shared/features/models.ts` 同时保留 `editable`、精确保存 `providerId` 和订阅 `selectedModel`；`ports.ts`、`queries.ts` 保留两套能力与各自 Query key；`feature-ports/models.ts` 的严格 snapshot/parser 与订阅委派并存；Browser Port 的绑定/恢复继续 native-only。

`selectedModel` 的旧 host 兼容可选形态没有被扩大为绑定成功：订阅 picker 仍要求回读值匹配 `providerId/modelId` 才确认新绑定。面板恢复后仍须快照中已无 managed provider 且 Managed Auth overview 回读成功，才交回普通 API 编辑。

## 直接修改文件

- `src/pages/models/OpenCodeModelsPanel.tsx`
- `src/shared/platform/tauri/features.ts`
- `tests/renderer/pages/models/OpenCodeSubscriptionRestore.test.tsx`
- `tests/renderer/pages/models/XaiSubscriptionSection.test.tsx`
- `tests/renderer/platform/xaiSubscriptionPort.test.ts`
- `tests/browser/support/features.ts`
- 本报告

订阅 fixture 补齐新版严格快照所需的 `editable`。OpenCode Port 回归改为从真实 `createTauriFeaturePorts()` façade 进入，覆盖两级延迟加载后方法仍可达，不能只验证底层工厂而漏过组装遗漏。

新增组合回归包含 builtin、两个同名但不同 ID 的普通 API provider，以及一个 editable=true 的托管 provider：确认托管条目不进入 API selector，托管期间普通写入阻断，builtin 模型仍只读；专用恢复与回读结束后，选择第二个同名 API provider，断言保存携带精确 ID 及恢复后的 revision，终态清除 API Key。既有 FDE 只读/切换草稿确认、订阅 revision/卸载后失败/双 owner 恢复回归保持通过。

## 验证

所有命令使用锁定 mise 环境和 `rtk`；依赖采用 FDE 合并后的 Vitest 4.1.11。

| 检查                                       | 最终结果                      | 证据                                                                      |
| ------------------------------------------ | ----------------------------- | ------------------------------------------------------------------------- |
| `mise run typecheck`                       | PASS，exit 0                  | `.trellis/.runtime/verification/integration-frontend-typecheck-final.log` |
| `mise run lint`                            | PASS，exit 0，无 warning      | `.trellis/.runtime/verification/integration-frontend-lint-final.log`      |
| `mise run format:check`                    | PASS，exit 0                  | `.trellis/.runtime/verification/integration-frontend-format-final.log`    |
| 定向 `mise run test:unit -- …`             | 5 文件、138 测试 PASS，exit 0 | `.trellis/.runtime/verification/integration-frontend-targeted-final.log`  |
| 所有直接修改源码/测试的 `git diff --check` | PASS                          | 本次工具回执                                                              |

定向测试文件为 `pages/models/Page.test.tsx`、`XaiSubscriptionSection.test.tsx`、`OpenCodeSubscriptionRestore.test.tsx`、`platform/featurePorts.test.ts` 与 `xaiSubscriptionPort.test.ts`（均位于 `tests/renderer/`）。最终测试在 React Compiler 修复后重跑。最后生产修改后不再追加功能。

## 交接与证据边界

本范围没有未修复的集成阻断项。根控负责最终源清单、route-chunk 图合约、全库 gate、生产 browser 与 GitHub CI；backend owner 负责原生 DTO、迁移与配置事务验证。本报告只证明上述源码与本地检查，不能替代最终远端 head 的绿色状态或真实账号/CLI UAT，也不宣称厂商订阅实际调用、额度或客户端重载已验证。

已向根控指出 Models 规范仍有“first provider”及质量规范的 Vitest 3 旧说明，由其统一处理文档。独立复核应重点看面板过滤/精确选择与恢复闭环，以及 Tauri 两级延迟加载的完整方法集合；本报告不替代独立评审。

> 归档说明：本文的时间与结论保留；具体工作站用户目录已替换为语义占位符。历史路径映射、原稿及提交稿哈希见[归档路径映射](research/archive-path-map.json)。可解析的相对链接已调整。

# 前端会话迁移独立复核（Grok 4.7）

范围：`src/pages/sessions/**`、`src/shared/features/session-migration.ts`、`ports.ts` 与 Tauri/browser `features` 里的 sessions 端口、`tests/session-migration/**`。对照 `implementation/frontend-final-fixes-brief.md`、`implementation/frontend-report.md`、`.trellis/spec/frontend/{reuse,type-safety,quality-guidelines}.md`、`.trellis/spec/backend/session-migration.md` 的渲染边界。未改生产代码和测试。`rtk mise run test:unit -- tests/session-migration`：6 个文件、38 项通过。报告里的 `pure-helpers.test.ts` 和 `validateBatchExportSessions(targets, previewSession)` 签名与仓库不符，不把它当证据。后端未冻结，不评其实现。

## P0

没有发现。

## P1

空的恢复结果被画成整批写入成功。`isAllCleanSuccess` 用 `restoreResult.every(...)`（`ImportPackageDialog.tsx:282-291`）。空数组的 `every` 为真，于是绿色横幅「恢复执行完成 / 已成功将 0 个会话写入」（`624-631`）。页面同一判断在 `Page.tsx:489-497`，空数组同样走到成功 toast「已处理 0 个会话记录，已写入本地存储」（`524-529`）。端口 schema 接受 `RestoreAttempt[]`，`[]` 能过校验（`sessionMigration.ts:68`）。触发：恢复调用正常返回、一条回执都没有。影响：没有写入却出现成功横幅和成功 toast。测试只锁了 failed / ambiguous / needsReconciliation 单条结果，没有空数组。最小修法：两处成功条件都加上 `length > 0`；空结果用失败或警告，文案写明没有回执，不调用成功 toast。两处判断应共用一个函数，避免只改一处。

## P2

1. 导出样例把连续用户和后一条答复编成同一轮。`buildPureTextPreview` 只在 `assistantFinal` 之后把轮次加一（`session-migration.ts:460-472`）。顺序 user、user、assistant 时，两条用户和那条答复都标「第 1 轮」。正文没有被删掉，也没有补一条假答复，但轮次标签把它们说成一轮。主会话流是分开的：上一条未答用户先落成未完成轮，再开始下一条（`Page.tsx:333-354`），末尾未答用户单独保留（`358-365`）。导入列表把 `messages.length` 印成「N 轮问答」（`ImportPackageDialog.tsx:506`），消息条数被说成轮数。最小修法：样例按消息顺序各占一块，未配答复的用户标未完成；导入列表改成「N 条消息」。导出集合本身仍是冻结的 `targets`，不要为这个标签去改导出 payload。
2. 未知阶段被说成已经提交写入。导入结果除失败和对账外，全部落到警告「会话已提交写入」（`ImportPackageDialog.tsx:646-652`），`ambiguous` 和 `packageVerified` 走这句。页面把 `ambiguous` 的说明也写成「会话已提交写入，仍在等待系统核验」（`Page.tsx:512-516`）。触发：返回这些阶段而不是 `nativeWritten`。影响：还不能确定的结果被说成写入已经发生。绿色横幅没有亮，所以不是 P1。最小修法：`ambiguous` / `packageVerified` 只说未确认、不要重试；`nativeWritePending` 才可以说已进入写入。`nativeWritten` 继续用成功色，这是 brief 要求的，不记缺陷。
3. 探针抛错被显示成「本地未安装」。`probeLocalProvider` 失败时写入 `installed: false`，`reasonCode` 被放进对象（`Page.tsx:120-128`）。`isProviderRestoreSupported` 在 `!installed` 时直接返回「本地未安装该客户端」，不读 `reasonCode`（`session-migration.ts:636-637`）。触发：探测 RPC 超时或返回无法通过 schema 的值。影响：确认按钮保持禁用，不会盲写；用户看到的原因是未安装。最小修法：探测失败单独占一个状态，展示 `parseMigrationError` 的句子，确认按钮仍然禁用。
4. 父组件每次渲染都重跑全组预览，复查期间确认仍可用。`onPreviewSession` 是内联函数（`Page.tsx:1370-1372`），effect 依赖它（`ExportPreviewDialog.tsx:123`）。重跑开始时不把 `loadingState` 置回 `"loading"`（`72-77`）。已经是 `"ready"` 时，确认条件 `loadingState !== "ready"` 不成立（`148`）。触发：对话框开着时页面重渲染（列表刷新）。导出 payload 仍是打开时冻结的 `targets`（`152-155`），集合不会偷偷换成后来的勾选。影响：第二次预览若会失败，用户仍能在它结束前点导出。最小修法：回调用稳定引用；effect 一开始设为 loading，这次跑完之前不能确认。
5. 同一来源的多条回执只显示数组里的第一条。目标身份匹配和来源匹配都是 `attempts.find`（`Page.tsx:286-301`）。触发：同一 `originId + snapshotId + provider` 有多条回执，前面一条是 `nativeWritten`，后面一条是 `needsReconciliation`。影响：对账条被挡住，页面像是只成功过。最小修法：多条命中时优先展示 `needsReconciliation`、`ambiguous`、`nativeWritePending`、`failed`，或标明不止一条。
6. 默认冲突文案承诺会打开已有会话，点击只调用恢复。文案是「跳过写入并打开已有会话」「直接定位已有会话」（`ImportPackageDialog.tsx:574-576`）。`handleExecuteRestore` 只提交 `restore`（`224-234`），不调用 `openRestoredSession`。最小修法：改成「沿用已有恢复记录，不重复写入」。打开会话仍走现有的单独动作。
7. 幂等键用了未选中的第一条摘要。`packageDigest` 取 `sessions[0].contentDigest`（`ImportPackageDialog.tsx:208`），再拼进绑定键（`session-migration.ts:505`）。勾选集合不变、只是第一条未选会话的摘要变了，就会换新 `requestId`。同一绑定参数重试仍复用 id，这点测试已锁，不重复记。最小修法：键里只放已选快照的 `contentDigest`（排序后），或包文件摘要。

## 可删的重复

- 成功判断在 `Page.tsx:489-497` 和 `ImportPackageDialog.tsx:282-291` 各写一份，空数组漏洞因此出现两次。
- `filterSnapshotsForProvider`（`session-migration.ts:653`）和 `validateBatchExportSessions`（`663`）没有调用方。对话框自己过滤 provider、自己循环 `canExportSession`。删掉这两个函数即可，不必再包一层。
- `canExportSession` 的注释写「未决或未闭合消息一律阻断」（`421-424`），函数只拒绝零条 user（`426-436`）。真正挡住未决终态的是预览抛错后整包不导出。注释过时；未答用户按契约应保留，不要把函数改成拒绝未完成轮。
- `StatusBanners` 的 `hasDuplicateCollision` / `onOpenDuplicateModal` 页面没有传入，碰撞条达不到。回执阶段已经表达对账，这条是死分支。

## 没有发现

- 同一绑定（路径、provider、快照集合、工作区、`requestKind`）重试复用 `requestId`；改 `requestKind` 会换新 id。对话框关闭后 `useDialogState` 换 key 重挂载。这些有测试。
- 混合包只列出当前目标 provider 的快照；目标不可写时确认按钮禁用。`grokbuild` 不是 `grok`。有测试。
- 打开导出时冻结当时的勾选；之后取消勾选，预览和 `export` 仍用那一组。某一条预览失败会点名并阻断整包。有测试。
- 主会话流不合并连续用户，不给未答用户补答复，也不把孤立 assistant 配一个假用户（`Page.tsx:325-368`）。`ConversationStream` 对未完成轮有单独卡片。
- `failed`、`ambiguous`、`needsReconciliation` 不是绿色成功。`nativeWritten` 用成功色，阶段条文仍是待续聊验证。列表扫描失败有「本地会话扫描失败」和重试，不是空列表冒充没有会话。
- 浏览器端口对 `pickPackageFile` / `pickExportPath` 仍是 native-only 拒绝。Tauri 端口对 invoke 结果做 schema 解析，未知字段进不了页面状态。
- 来源匹配带 `originId + snapshotId + provider`，目标匹配带 `targetNativeId + provider`。缺的是多条命中时的选择，不是把来源 ID 当成目标 ID。

## 证据限制

浏览器全量点击由 QA 在做，这次没有开浏览器。空数组、探针抛错、预览 effect 重入没有现成测试，结论来自上述分支。未读用户会话和凭据，未做真实推理。后端最终复核不在这一轮。

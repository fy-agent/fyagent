# 验证迟到响应最小修复

针对组合审查 `integration-stage-0a08c38f.md` 的 P2。只改两个指定产品文件和一份定向 renderer 测试；未提交，未修改其他执行者文件。

## 实现

- `src/shared/features/verification/VerificationPanel.tsx`：沿用包面板已有的实例 live ref + effect cleanup。所有 await 后的共享 Query 写入、错误后的 invalidate、预览/保存/取消提示均先检查所属实例仍然有效。旧 revision 或旧项目的已卸载实例不能写回，也不能把新快照 invalidate。
- `src/app/ProjectsWorkspace.tsx`：为交付 adapter 建立按 projectId/revision/可操作状态重建的局部 DeliverySession。recordLocalFixture 在项目回读后和 native run 返回后检查 session 是否仍有效；bind 后的验证刷新也不从旧 session 触发。保留原生已完成操作，仅拒绝过时的 UI/缓存写回。
- 没有引入新状态框架、改 cache key 或扩大后台取消语义。

## 定向证据

新增 `tests/renderer/app/ProjectsWorkspace.test.tsx`，使用真实 ProjectsWorkspace composition 和验证面板，仅替换项目选择壳/包 UI 与 native ports，控制 Promise 返回顺序。覆盖：

1. 验证面板 A rev1 结果迟到，不能覆盖 A rev2 已读到的 stale。
2. 验证面板 A 结果迟到，不能在切至 B 后重新写入旧 A 缓存。
3. recordLocalFixture A rev1 结果迟到，不能覆盖 A rev2。
4. recordLocalFixture A 结果迟到，不能污染切项目后的缓存。
5. rev1 迟到异常不能 invalidate rev2 缓存或向新面板显示错误。

检查结果：

- `mise run test:unit -- tests/renderer/app/ProjectsWorkspace.test.tsx tests/renderer/features/VerificationPanel.test.tsx`：2 文件、11/11 通过。
- `mise run typecheck`：通过。
- 两产品文件和新增测试的定向 ESLint：通过。
- 指定文件 format 和 diff check：通过。

首次新测试失败是 fixture 在面板时钟捕获之后生成 recordedAt，触发已有“未来记录待复核”规则，未进入迟到响应步骤。已将 fixture 记录时间设为已发生时间，未延长等待预算；之后全部场景通过。

本次为 renderer 组合回归，不重复全库、原生构建或本机 UAT。两个产品文件与测试 ownership 交回 root，供独立复核和最终统一验收。

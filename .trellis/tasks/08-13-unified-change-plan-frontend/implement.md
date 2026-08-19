# 统一配置变更前端实施计划

本文件规划后续实现，不代表当前子任务已实施。每一阶段应独立提交；任何阶段失败时可回滚该提交，不跨阶段混合生产代码与治理文档。

## 阶段 1：共享合同与只读预览壳

- [ ] 与后端方案对齐 plan/job/readback/recovery 的受控 union 与 presentation code。
- [ ] 新增薄 `change-plan` API/query 层，不迁移既有领域 queries。
- [ ] 建立 `ChangePlanFlow`、预览摘要、风险、恢复和证据说明组件。
- [ ] 建立 Provider/WorkBuddy 两个显式 projection，无动态 registry。
- [ ] 补充 i18n 文案与类型级 exhaustive handling。
- [ ] 提交：`feat(change-plan): add shared preview flow`。

验收：fixture plan 能渲染两个领域的 ready/stale/expired；无写入 mutation；secret 不出现在 DOM fixture、query key 或日志。

## 阶段 2：Codex Provider 首条纵切

- [ ] Add Provider 校验后生成 create plan。
- [ ] Edit Provider 校验后生成 update plan。
- [ ] Provider Card 切换生成 switch plan。
- [ ] 统一一次确认后创建 apply job。
- [ ] 保留返回修改时的本地 draft。
- [ ] 移除这三个入口中的直接 mutation 成功 toast，结果只由 job snapshot 驱动。
- [ ] 重启建议在回读成功后作为独立动作展示。
- [ ] 提交：`feat(codex): route provider changes through change plan`。

验收：create/update/switch 均经历 preview -> confirm -> running -> readback result；additive 与其他 AppType 不进入此纵切；`liveConfigChanged` 不被当作完成证据。

## 阶段 3：执行进度与结果恢复

- [ ] 实现 `useApplyJobQuery` 与 event subscription；query snapshot 为权威。
- [ ] 支持重载/重新进入恢复运行中和 terminal job 展示。
- [ ] 渲染 warning、partial、recovered、recovery failed 和逐资源结果。
- [ ] terminal 后按受控 resource code invalidate 领域 query。
- [ ] 增加轻量运行中 job 返回入口，仅保存 job ID。
- [ ] 提交：`feat(change-plan): add durable apply progress and results`。

验收：丢失 event 后 query 可恢复；重复 event 不造成重复 toast/状态跳转；刷新页面不把 running 误报为失败；partial 不显示全局成功。

## 阶段 4：WorkBuddy 接入

- [ ] WorkBuddy 编辑校验后生成 update_models plan。
- [ ] 保留远程模型获取为编辑期独立动作。
- [ ] 移除 renderer overwrite token、`PendingOverwriteSave` 和专用二次确认路径。
- [ ] revision 漂移统一进入 stale/重新预览。
- [ ] 展示真实 readback、自动恢复与恢复失败结果。
- [ ] 保留 status/model IDs query 的脱敏缓存边界。
- [ ] 提交：`feat(workbuddy): adopt unified change plan flow`。

验收：用户只确认一次；无强制覆盖；fetch 不出现在 apply job；API Key、原始 models.json、完整路径与内部 capability 不进入 renderer cache。

## 阶段 5：无障碍、响应式与文案收口

- [ ] 覆盖键盘、focus return、focus trap、aria-live、alert 去重。
- [ ] 覆盖窄屏、长 model ID、长本地化文案、减少动态效果。
- [ ] 审核所有 success/ready/verified 文案，确保应用证据与使用证据分离。
- [ ] 审核 unknown backend code 的安全 fallback。
- [ ] 提交：`fix(change-plan): harden accessibility and evidence copy`。

验收：全键盘完成预览与确认；屏幕阅读器不被步骤 event 连续打断；颜色不是唯一状态信号；无证据固定显示“尚未观察到真实使用”。

## 建议验证命令

由后续实现任务根据项目脚本确认准确命令后执行：

```bash
rtk npm run lint
rtk npm run typecheck
rtk npm run test -- change-plan
```

还需运行针对入口组件的测试，覆盖 Provider create/update/switch、WorkBuddy stale、partial 与恢复失败。当前规划子任务不运行测试。

## Review gate

- [ ] 后端 DTO 与前端 union 一致，未知状态 fail closed。
- [ ] 没有直接 mutation 的旁路。
- [ ] 没有 secret/capability 进入 plan projection、query key、持久缓存、toast 或错误详情。
- [ ] Provider 与 WorkBuddy 只共享体验，不共享领域写入语义。
- [ ] 所有完成声明由 job + readback 证据支持。
- [ ] 每阶段单独提交，提交范围不包含其他 Agent 的文件。

## 回滚点

- 阶段 1 可单独删除未接入口的共享组件。
- 阶段 2 通过领域 feature gate 回退 Codex 入口；不得静默回到无确认写入，回退时明确显示版本不支持。
- 阶段 3 可关闭 event 优化并保留 snapshot polling。
- 阶段 4 仅在 WorkBuddy 后端完整支持 plan/readback/restore 时上线；否则保持未接入状态。


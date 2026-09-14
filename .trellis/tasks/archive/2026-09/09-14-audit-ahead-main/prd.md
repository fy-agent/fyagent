# 审计本地领先提交并合并 main

## Goal

审计 dev/laiyongjie 相对 origin/main 的全部领先提交，重构与补充相关 Trellis SPEC，完成全量验证、PR 创建、CI 修复与合并。

## Requirements

- 审计基线固定为任务开始时的 `origin/main..dev/laiyongjie` 六个提交，区分两条既有工作流：
  1. Claude/Grok CLI npm 最新版本解析、发现与 Windows 开发态执行修复；
  2. 二级页面精简、Renderer/浏览器测试与仓库契约稳定性修复。
- 不新增产品能力，不改变已评审的用户流程；发现缺陷时只做与上述提交直接相关的修复。
- 逐项核对代码、测试、任务归档与 `.trellis/spec/**` 是否一致，消除同一契约在多个 SPEC 中的重复所有权。
- 对超过 Trellis 默认 `context_injection.max_file_bytes=32768` 的受影响 SPEC 做语义拆分；保留稳定入口与显式交叉引用，不通过提高注入上限或删除安全/回滚契约解决。
- 受影响的基础设施、跨层和平台契约必须保留可执行签名、验证/错误矩阵、Good/Base/Bad、测试断言和 Wrong/Correct 示例。
- 更新 backend/frontend 索引及所有本次变更涉及的有效 SPEC 引用；历史归档材料仅在出现失效的活动路径时调整，不做无意义批量改写。
- 运行与 CI 对齐的完整质量门禁；任何失败都要定位根因并补回归测试或契约，而不是放宽预算、跳过检查或隐藏警告。
- Trellis 任务在“可交给远端合并执行器”的边界关闭：完成直接会话预归档、工作提交、任务归档、journal、post-archive 校验与 exact-head 只读复核。
- 任务归档后，从 `dev/laiyongjie` 向 `main` 创建 PR，等待所有必需检查通过，按仓库合并治理完成合并；若 CI 失败，修复后重新走适用的本地/Trellis readiness 流程。
- 合并后确认远端 `main` 包含 PR 结果，并保持受保护的 `dev/laiyongjie` 分支可继续使用。远端 PR/队列/合并读回属于 post-archive merge executor 证据，不伪装成任务归档前已经完成。

## Acceptance Criteria

- [ ] `origin/main..HEAD` 的每个提交和所有改动文件均已审查，审查结论写入任务材料。
- [ ] 受影响 SPEC 的语义所有权清晰；所有被任务上下文引用的 owner 文档不超过 32768 bytes。
- [ ] 新拆分文档已加入 backend/frontend 索引，原入口保留准确路由，不存在相互矛盾或复制粘贴的规则。
- [ ] Claude/Grok live npm、PATH-default 发现、npm 12 allow-scripts、Windows LocalProcess、Renderer 精简与测试稳定性均有对应自动化断言。
- [ ] Trellis 任务 `validate`、契约检查、全仓检查和本次变更对应的浏览器/性能门禁通过。
- [ ] 工作提交、任务归档和 journal 顺序符合 Trellis；post-archive 校验通过，exact PR head 已冻结且工作树干净。
- [ ] `research/merge-handoff.md` 明确记录 post-archive PR、exact-head CI、Merge Queue 和最终 `main` 读回要求，且这些步骤不会被误写为任务归档前证据。

## Notes

- 用户在本轮请求中已明确授权完成审计、SPEC 调整、PR、CI 修复和合并，不需要另行扩大功能范围。
- 初始工作树干净；`dev/laiyongjie` 相对 `origin/dev/laiyongjie` 领先四个提交，相对 `origin/main` 领先六个提交。
- 用户级工作在远端 merge/readback 后结束；Trellis 任务按 `github-merge-governance.md` 的强制顺序先归档，再由同一会话继续执行 post-archive merge handoff。

# 组合审查两项修复闭合

最终核对对象：`d2a5dc7d`，前序审查对象为 `0a08c38f`。先读定向工作树修改，root 冻结后以限定四个源码/测试文件的 `git diff d2a5dc7d -- ...` 确认零差异。仅核对已提出的 P1/P2，没有扩展评审面、修改产品或重复全库。

结论：两项发现均在修复代码和定向测试内容层闭合，未发现新增具体阻断。最终统一 native/frontend 门禁由 root 完成；本报告不替代这些运行结果。

## P1：Codex 检查地址选择——已闭合

`src-tauri/src/services/provider/mod.rs` 的 Codex 分支已使用现有 `codex_config::extract_codex_base_url`，删除全文正则和无用 Regex import。返回值另拒绝空白 URL；活动路由缺失、只有非活动节/注释 URL、无效 TOML 均走既有封闭错误，不再向第一处文本 URL 发送凭据。

新增 `extract_codex_credentials_uses_active_route_and_ignores_inactive_or_commented_urls` 调用正式 ProviderService extractor：正例在活动节之前放置注释和非活动节并断言活动 URL/key；负例覆盖缺失活动路由、仅注释、空 URL、无效 TOML。该回归与原发现直接对应，没有另造 parser，也没有改变其他 Agent 的解析策略。

本轮独立审查未重新运行 Rust。该用例的最终 native 通过结论由 root 的冻结后门禁提供。

## P2：旧项目修订迟到响应——已闭合

- `src/shared/features/verification/VerificationPanel.tsx` 的实例 live ref 在 cleanup 后失效；成功缓存写回与失败 invalidate 均先检查 live。异步预览、保存和取消消息也不再写已卸载实例。
- `src/app/ProjectsWorkspace.tsx` 的 DeliverySession 随 projectId/revision/可操作状态重建；项目回读和 native run 返回之后均检查会话有效性。旧包检查不会写共享验证缓存；旧 bind 结果不会触发新会话刷新。没有篡改已经完成的 native 事实或引入新的后台取消语义。
- `tests/renderer/app/ProjectsWorkspace.test.tsx` 的 5 个受控 Promise 场景使用真实 workspace composition 和验证面板，覆盖 panel/kit 两条路径、改 revision/换项目、迟到异常；明确让 rev2 快照先进入缓存，再释放 rev1 响应，断言 revision、缓存、可见“待复核”和 invalidate 状态。测试不是只检查 key 或字符串。

执行者证据见同目录 `evidence-late-response-fix.md`：2 文件 11/11 renderer 用例、typecheck、定向 ESLint、format 和 diff check 通过。独立复审已核对测试内容及其与最终冻结文件的一致性，未重复运行。首轮 fixture 使用晚于组件捕获时钟的 recordedAt 被真实未来时间规则拦截；改为已发生时间后再执行，未放宽等待或产品判定。

## 范围与证据边界

此轮只读检查两项修复及对应回归，没有再审其他领域、触发真实模型调用、改用户配置或运行发布。本 Agent 的定向 `git diff --check` 通过。两项原发现可标为 closed；本机桌面流程、统一门禁、真实外部服务和平台验证仍以 root 的最终证据为准。

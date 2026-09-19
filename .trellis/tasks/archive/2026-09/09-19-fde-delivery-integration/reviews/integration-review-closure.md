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

## 最终追加：旧 schema 与延迟 adapter 的只读复审

依据 root 新授权，对冻结待提交的两个局部修复及对应测试只读核对；没有运行测试或修改产品。结论：**无新增阻断**。

- `database/dao/projects.rs` 的跳过条件同时要求 kind=skill、user_version<3、Skills 缺少 id；只将尚未具有当前身份的旧 Skills seed/trigger 延后，不吞掉 schema≥3 的缺列或 SQL 错误。旧迁移依次执行 Skills 重建，21→22 再调用同一 project schema owner 补 seed 和四个触发器，当前已有代际仍由 INSERT OR IGNORE 保留。
- `database/tests.rs` 的真实旧 schema fixture 按启动顺序 create→migrate，检查迁移前零 Skill trigger、迁移后四个，再实际 INSERT/UPDATE 得到 generation=2，重开不重置；额外负例把旧 Skills 冒称 current schema，要求失败。`backup.rs` 的旧 SQL 导入测试补查 provider 更新确实增长代际、Skill 四个 trigger 已补齐。测试覆盖实际迁移/写入行为，没有以表存在替代身份跟踪。
- `shared/platform/tauri/features.ts` 延迟加载五个模块：projects、delivery-kits、verification、models、configRecovery。逐方法转发保持原参数顺序及全部 rest 参数，未丢失 recover、revision、preview、凭据参数或 overwriteToken。cancel 的布尔结果、导出取消 false、异常 rejection 均原样穿过，原 port parser 和 native owner 仍负责校验。
- 已读取五个真实 factory：其返回对象没有跨调用的可变实例状态；verification 内部 snapshot helper 也是无状态校验。取消、预览 token 和覆盖 token 的权威在 native/request 中，因此每次调用新建薄 adapter 不会丢失操作状态。未新增凭据缓存或复制执行器。
- `deferredProjectPorts.test.ts` 检查首次使用前模块未加载、重复调用使用模块缓存、原参数/异常对象/取消结果转发；现有 `featurePorts.test.ts` 仍调用实际 Tauri factory 并校验模型凭据和覆盖 token 的 native payload。此处只确认测试内容与实现相符，最终运行结果由 root 门禁提供。

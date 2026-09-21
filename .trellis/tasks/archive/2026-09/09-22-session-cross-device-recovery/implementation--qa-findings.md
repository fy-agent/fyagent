> 归档说明：本文的时间与结论保留；具体工作站用户目录已替换为语义占位符。历史路径映射、原稿及提交稿哈希见[归档路径映射](research/archive-path-map.json)。可解析的相对链接已调整。

# QA 执行发现

## QA-FID-005：真实缺答复被误判为不可导出（已修复并复测）

- 复现：`rtk mise run test:unit tests/session-migration/schema.test.ts`
- 断言：有序消息追加 `userText(seq=2)`，`openUserMessages=[2]`，没有任何未知 assistant。
- 初次结果：`canExportSession` 返回 `allowed=false`，理由称“无法判定是否为最终答复”。
- 合同：PRD R1 与 test-plan §1/§4 明确区分“没有 assistant”与“assistant final 不可判定”；前者保留原位置并可导出，后者才 `finalAnswerIndeterminate`。
- 修复后证据：前端 owner 将 `openUserMessages` 保留为未完成状态，不再作为导出阻断；`rtk mise run test:unit tests/session-migration/schema.test.ts` 为 12/12 通过。

## QA-PKG-003：前端包 schema 未锁定 v1（已修复并复测）

- 复现：同上。
- 断言：把合法包 `schema` 改为 `fyagent.session.v2`。
- 初次结果：`sessionPackageSchema` 使用 `z.string()`，解析成功。
- 合同：wire contract 与 test-plan §5.4/§5.5 要求未知 schema/version 返回 `packageSchemaUnsupported`；前端 Type Safety spec 要求新 native response 边界拒绝未知协议版本。
- 修复后证据：前端 owner 改为 `z.literal("fyagent.session.v1")`；同一命令 12/12 通过，后端权威门控仍需 Rust 测试。

## 已通过的同轮断言

- envelope/exporter/session/origin/message/extraction/omitted 七层 excess-field 均拒绝。
- 未知 message kind 拒绝。
- CRLF、emoji、Mac/Windows 路径与代码块逐码点保留。
- userAttestation 与系统 stage 分离。
- disabled runtime probe 的 `writeSupported=false` 不因用户自报对象或目录对象改变。

## QA-UI-001：浏览器验收已进入执行，既有 shell 断言未同步

- 复现：`rtk mise run test:browser`
- 当前结果：production boot 3/3 通过；主套件 647 通过、20 失败。
- 其中 8 个失败是 `tests/browser/shell.spec.ts` 在四个视口重复：新增会话入口后仍期待 6 个主导航项和 12 个键盘控制，实际为 7/13。既有测试不在 QA writer 范围，QA 未修改。
- 会话迁移文件初轮的 12 个定位器失败已按当前可访问树修正；定向复测为 8 通过、4 个真实 capability 门控失败，见下一项。

## QA-IMP-008：disabled local probe 仍允许点击恢复

- 复现：
  `rtk mise run test:browser -- tests/browser/session-migration.spec.ts`
- fixture 明确让 `probe_local_provider` 返回 `installed=true`、`writeSupported=false`、`reasonCode=providerVersionUnsupported`，没有依赖 default delegate。
- 页面正确显示“当前版本的恢复能力尚未验证”，但“在目标软件中恢复”按钮仍为 enabled；四个 Chromium 视口均失败。
- 影响：用户自报前已经存在 capability 旁路，无法继续证明自报后仍不提升能力。
- 边界：这是生产 `SessionsPage` 按钮门控缺陷；QA 只保留失败断言，不修改生产实现。

## QA-BE-001：Rust 最终复跑仍等待后端验收接口

- 上一轮复现 `rtk mise run rust:test session_migration_` 时的中间态编译错误：
  - `src/database/dao/session_restore.rs` 引用 `crate::session_manager::migrate::model`，但 `src/session_manager/mod.rs` 尚未声明 `migrate`。
  - DAO 使用不存在的 `AppError::NotFound`。
  - `an_unknown_stored_stage_is_an_error_not_a_silent_default` 中的 `lock_conn!` 展开含 `?`，测试函数返回 `()`。
- 当前观察：`session_manager::migrate` 已接线，但 `backend-exit.json` 尚未出现，也没有迁移模块的 `test-hooks` 窄 re-export。
- 影响：按协调约束不反复编译根级后端，也不把 path-include 假 crate 迁移成不存在的公共接口；Rust 扩展集仍记为 blocked。
- 边界：上述生产文件不在 QA writer 路径，QA 未修改。

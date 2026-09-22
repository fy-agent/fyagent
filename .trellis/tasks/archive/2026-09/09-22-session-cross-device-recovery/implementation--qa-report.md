> 归档说明：本文的时间与结论保留；具体工作站用户目录已替换为语义占位符。历史路径映射、原稿及提交稿哈希见[归档路径映射](research/archive-path-map.json)。可解析的相对链接已调整。

# Session cross-device recovery QA 执行报告

## 范围与结论

本轮只写入 QA owner 允许路径，使用合成 Codex JSONL、fake Tauri IPC 和临时目录；没有读取或写入真实用户会话、凭据、provider 存储，也没有发起模型请求。

47 条用例没有被整体宣称通过。当前已形成可执行的 fixture、前端 schema/port/UI、Rust 生产 extractor/package 边界和浏览器页面验收五层入口；command 编排、receipt/native 故障注入、跨系统与真实 provider 验证仍按 `qa-case-map.md` 逐条保留缺口。

## 实际新增

- `tests/session-migration/fixtures/*.jsonl`
  - 三轮 final-only/六类泄漏哨兵。
  - assistant final 缺少 `phase`。
  - 连续 user、中断后重问、尾部未完成 user。
- `tests/session-migration/fixture-contract.test.ts`
  - 冻结合成源 fixture 的语义，避免测试输入自身漂移。
- `tests/session-migration/schema.test.ts`
  - v1 闭集 schema、七层未知字段、未知消息类型、正文逐码点保真、真实缺答复、用户自报与 capability 分离。
- `tests/session-migration/tauri-port.test.ts`
  - restore request 闭集、禁止 overwrite/command/argv、同 request payload 重放、native reply 严格解析、专用导入/导出文件 picker 命令。
- `tests/session-migration/ui-state.test.tsx`
  - 系统 stage 与用户自报分离、disabled capability 不提升、未知副作用禁止盲重试、missing 与 indeterminate 区分。
- `tests/session-migration/import-dialog-behavior.test.tsx`
  - 超时重试 requestId 复用、另存绑定生成新 ID、pending 重复点击合并、关闭/重开隔离、多 provider snapshot 过滤、probe 门控、非成功 stage 与 JSON 字符串错误展示。
- `tests/session-migration/page-behavior.test.tsx`
  - 多选逐项 preview、任一失败整体阻止、冻结预览集与最终导出集一致、ambiguous 不发成功 toast。
- `src-tauri/tests/session_migration_model.rs`
  - 直接执行当前生产 `model.rs`、`identity.rs`、`package.rs` 和 Codex extractor。
  - 覆盖重复 key/尾随文档、digest 篡改、未知字段/消息类型、原文往返、unknown final/version fail-closed、运行时注入排除、连续 user 顺序、同正文不同来源 snapshot 分离、不确定副作用阶段阻断。
- `tests/browser/session-migration.spec.ts`
  - preview 按 sourcePath 返回不同 origin/snapshot；probe 明确 `writeSupported=false`；覆盖同正文双来源、连续/未完成 user、capability 不提升和旧 `/memory` 入口。

## 执行证据

| 命令 | 结果 | 解释 |
|---|---|---|
| `rtk mise run test:unit tests/session-migration` | pass：7 files，45 tests | fixture/schema/port/component/page 全部通过 |
| `rtk mise run typecheck` | pass | 严格 TypeScript 类型检查通过 |
| `rtk mise run rust:test session_migration_`（基础集） | pass：7 tests | schema、parser、digest、stage 基础合同通过 |
| 同命令（加入生产 Codex extractor 后首次） | partial：10/11 | 唯一失败为 QA harness 的共享临时文件并发竞争；已改为每用例独立 `tempdir` |
| 同命令（隔离修复后） | blocked | 当前无 `backend-rework-exit.json` 和迁移 `test-hooks` re-export；按约束未反复编译 |
| `rtk mise run test:browser -- tests/browser/session-migration.spec.ts tests/browser/shell.spec.ts` | partial：boot 3/3；主套件 31 pass / 5 fail | 4 个迁移失败是当前文案定位器漂移，已修并单独复测；剩余 1 个为 900×600 shell 底部溢出 |
| `rtk mise run test:browser -- tests/browser/session-migration.spec.ts`（定位修正后） | pass：boot 3/3；迁移 12/12 | disabled probe、用户自报和 workspace 选择均不能提升能力 |
| `rtk mise run lint`（上一轮） | fail：2 errors | 当时均位于实现 owner 的 `src/pages/sessions/Page.tsx`；本轮协调要求未包含 lint，因此未据旧结果声称当前仍失败 |

浏览器已进入真实页面执行；最终 Rust 复跑仍未开始。两者不能被合并描述为整体验收通过。

## 已确认并回归

- 前端 v2 schema 曾被接受，owner 改为 literal v1 后，相关单测已转绿。
- 真实缺答复曾被 `canExportSession` 误阻断，owner 修正后，未完成 user 可保留的单测已转绿。
- route chunk 和初始 JS 预算问题已修正；production boot 3/3 通过。
- 组件/page 行为已确认 requestId、重复点击、异步生命周期、多 provider 过滤、非成功 stage、批量冻结集合和专用文件 picker。
- disabled capability 页面旁路已由 owner 修复；真实页面在四个视口均保持恢复按钮 disabled，用户自报后也不提升能力。
- Rust 集成测试函数已使用 `session_migration_` 前缀，`mise` 的 cargo filter 能实际执行这些测试，不再出现“0 tests”假绿。

## 未解决项与复现

1. Rust test-hooks / 完成标记
   - 当前 `session_manager::migrate` 已接线，但没有 `backend-rework-exit.json` 或迁移模块窄 re-export。
   - QA 未把 path-include 假 crate 迁移成不存在的库接口，也未反复触发根级编译。
2. 900×600 shell 底部溢出
   - 复现：`rtk mise run test:browser -- tests/browser/session-migration.spec.ts tests/browser/shell.spec.ts`
   - `/sessions` 已正确进入导航合同和键盘顺序；最小视口中最后一个主导航控件底边为 605px，超过合同上限 601px。其余三个视口及 shell 顺序用例通过。
   - 生产布局不在 QA writer 范围；断言保持严格，未启动完整 browser。
3. command/receipt/native 注入面
   - 当前没有稳定公开的 command orchestrator、counted writer、receipt fault/crash-point 接口，因而 same-request writer=1、unknown side effect writer=0、旧 memory 字节/权限/mtime、目录映射 native readback 等不能伪造为已验证。
4. 物理机与 provider
   - Windows↔macOS、安装版 provider G0–G7 和真实续聊仍需明确测试环境；本轮按约束未调用真实模型。

完整逐 case 状态见 `qa-case-map.md`，接口请求见 `qa-interface-request.md`，确定性问题见 `qa-findings.md`。

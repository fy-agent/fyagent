# QA 跨包接口需求

## 事实

- QA writer 只能新增 `src-tauri/tests/session_migration*.rs`，不能把测试塞进生产模块。
- 当前 `src-tauri/src/lib.rs` 将 `session_manager` 声明为私有模块；现有集成测试无法调用迁移纯函数或注入 native writer。
- 仅做源码文本扫描不能证明 final-only、严格包解析、身份摘要或不确定副作用重放行为。

## 最小请求

请后端 owner 在不暴露 Tauri 任意能力的前提下，为 Rust 集成测试提供以下最小可调用面之一：

1. **推荐：窄 re-export**  
   在 crate root 仅 re-export 纯函数/DTO与测试所需的注入式编排入口：严格包解析、Codex fixture 提取、content/snapshot/slot 计算，以及接收 fake native adapter / isolated receipt store 的恢复编排。生产副作用实现保持私有。
2. **备选：`test-hooks` feature**  
   在已有 `test-hooks` Cargo feature 下 re-export 同一组入口；`mise run rust:test` 必须启用该 feature，否则本任务集成测试不会实际编译运行。
3. **不接受作为唯一方案：源码扫描**  
   可补合同检查，但不能替代行为测试。

若后端只暴露严格包/提取/identity，而恢复编排不可注入，QA 将把 same-request 副本重放、不确定副作用禁止重试、memory sentinel 标为 blocked，并给出具体缺口，不声称通过。

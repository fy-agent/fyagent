# 执行计划

- [x] 刷新 main，确认开放 PR，建立独立分支和工作树。
- [x] 读取当前 feature/文件恢复/开发环境合同，创建规划与回退说明。
- [x] 完成相关历史/测试复用检查，审阅最小变更边界。
- [x] 在原有模块补回归用例，先跑旧实现并保存预期失败。
- [x] 实现 Prompt 状态转换门禁与 Daily list/search/CRUD 统一有效日期准入。
- [x] 修复后运行相同回归；检查 enabled 更新/显式停用和合法日记行为。
- [x] 同步最小现行合同/维护文档，运行格式、Rust、相关 renderer/架构与仓库检查。
- [x] 构建本分支原生测试产物，在隔离配置运行两个 GUI 场景并回读。
- [x] 独立审阅变更，修复同秒显式导入ID碰撞并补确定性回归，重验最后改动。
- [x] 恢复日常应用，比较真实资料摘要，记录检查与剩余范围。
- [x] 提交本分支并核对提交内容；不混入其他任务，不合并 main/正式发布。

## 验证入口

由仓库 mise API 执行：mise run rust:test <filter>；mise run rust:fmt:check；mise run rust:clippy；mise run test:unit -- <focused paths>；mise run check:prearchive --exclude-active-task .trellis/tasks/09-08-issue-141-prompt-memory-closeout；mise run build:debug（本机独立调试包，复用受控测试编译输出；非正式发布，不替换安装版）。
测试使用 FYAGENT_TEST_HOME，运行多个使用全局测试 home 的 Rust 用例时遵守现有 serial/测试目录机制。先精确失败/成功检查，再执行所需全门禁；无新修改不重复整套测试。

## 回退检查点

1. 回归用例提交/记录能独立证明旧行为失败。
2. 修复改动只覆盖既有所有者，避免混入重构。
3. 原生测试不安装到 /Applications，保留旧应用可立即恢复。
4. 最终提交为回退单元；本任务只记录回退方式，不实际回滚其他人的改动。

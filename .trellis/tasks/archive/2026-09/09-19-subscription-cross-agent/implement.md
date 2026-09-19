# 实施计划

1. 固定最新主线与共享目录基线，读取当前 owner 合约，保存调查结论。
2. 后端补齐 OpenCode 精确账号绑定、revision 准入、专属路由、原生配置/回读与代理恢复，并添加行为回归。
3. 前端复用订阅选择区，增加 OpenCode 窄 port、严格解析、写入确认和 snapshot 回读；保留未知状态阻止与 API Key 流程。
4. 主控整合检查；独立 review 覆盖所有本次修改，修复后执行最终检查。
5. 执行 focused subscription/OpenCode Rust 与前端回归，再执行项目要求的完整 check、浏览器检查；记录真实证据边界。
6. 更新最小 owner SPEC，提交并推送独立分支；核对共享目录未变化，交付结论与使用入口。

## 分工

- 调研 Agent：仅 `research/current-chain.md`。
- 后端执行 Agent：`src-tauri/**` 与后端必要契约/测试；不修改 renderer。
- 主控：前端、任务设计与整合验证；保留唯一最终验收责任。

## 检查入口

`mise run rust:test -- subscription`、`mise run rust:test -- opencode`、相关 `mise run test:unit -- <files>`、`mise run check:prearchive -- --exclude-active-task .trellis/tasks/09-19-subscription-cross-agent`、`mise run test:browser`。真实账号调用只在可用且已授权的独立验证条件下开展。

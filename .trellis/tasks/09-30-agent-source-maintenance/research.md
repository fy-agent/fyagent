# 实证与待核实信息

检查日期：2026-09-29。这里记录来源与用途，不把上次聊天的数字视为永久锁定值。

## 本轮已读取的一手来源

- main 基线及保护检查：
  https://api.github.com/repos/fy-agent/fyagent/branches/main
  `1a4a1cfa2790528a989995fac50469061a4d78ae`；必需 context 为 `CI / Required`。
- 源版本查询文件：
  https://github.com/fy-agent/fyagent/blob/1a4a1cfa2790528a989995fac50469061a4d78ae/src-tauri/src/services/tooling/versions.rs
  精确 blob `173e7836410ebeadd55f710c6e3ead57b38e37b0`。
- Hermes 官方版本元数据：
  https://github.com/NousResearch/hermes-agent/blob/v2026.9.24/pyproject.toml
  `[project].name = hermes-agent`；`[project].version = 0.21.5`。
  这是该 tag 的固定快照，不是对未来 latest 的承诺。
- OpenCode 官方下载页：
  https://opencode.ai/download
  Terminal 显示 `@opencode/cli`、`/v2/install`；仍单列 Desktop。新版稳定状态与本机渠道识别未验证。
- 发布约束：
  https://github.com/fy-agent/fyagent/blob/1a4a1cfa2790528a989995fac50469061a4d78ae/.trellis/spec/backend/github-release-workflow.md
- GitHub 授权错误解释：
  https://docs.github.com/en/rest/using-the-rest-api/troubleshooting-the-rest-api#resource-not-accessible
  `Resource not accessible by integration` 指向当前连接 token 权限不足。重复口头确认不会修改上游 scope。

## 不能据此下结论的项目

- 本轮未读出 PyPI 实时 JSON；上次聊天所述 `0.19.0` 必须在实施前重新查询。
- 各第三方镜像当前同步度、各桌面完整包版本及签名没有在本轮完成联网验证。
- 当前未复读 #198 的最新状态；重新使用该 PR 前需重新查询和审查。
- 代码 fixture 里的版本号不得直接被归类为“生产版本过时”。
- 没有 Rust 编译器、Windows/macOS 原生验收环境；新增 Rust 测试尚未运行。
- 当前 sandbox 的 DNS 失败不能证明外部服务故障。

# 三 PR 本地集成、编译与体验验收

## Goal

将三个开放 PR 中仍有价值的行为整合为可追溯的 Windows x64 体验候选；本机先验证，交给维护者补 macOS 原生证据并按仓库流程合并。

## Requirements

- R1：本地集成 [#443](https://github.com/fy-agent/fyagent/pull/443) 的 IPC/ACL 清理和 [#444](https://github.com/fy-agent/fyagent/pull/444) 的 WSL/SSH/Keenable/模型改动。对 [#198](https://github.com/fy-agent/fyagent/pull/198) 按行为判重，拟只移植仍缺失的修复与回归，不导入过时实现或重复修复。原工作区用户改动不动。
- R2：用命令行验证集成提交、记录实测和人工阻塞。允许子代理并行审查/分析，但重任务串行且做 CPU、物理内存、commit 和磁盘监控。每次编译结束先封存产物再清本轮独占缓存；WSL 原生测试后只对专用测试 WSL2 VHDX 按安全门禁压缩。
- R3：本机不做 macOS 原生验收；共享代码及 macOS 行为必须保留。按用户本轮授权发布候选和验收 Issue，由 `@nonghua123` 按确切候选 SHA 验证；收齐证据后按仓库保护流程合入，不关闭 #198、不直接推送 main。

## Acceptance Criteria

- [x] AC1（R1）：四个输入、两次合并及 #198 的行为取舍已记录；候选提交及远端完整 Git 树逐项匹配，见 [执行记录](execution-result.md#远端交付检查点)。
- [ ] AC2（R2）：命令、正向/负向用例、退出码与日志可定位；执行 `mise run check`，拒绝零测试假通过，区分模拟、原生 fixture 与真实连接。交付 Windows x64 exe、SHA-256、日志和桌面启动结果；H1 可在日常普通账户验收，不要求 VM。资源峰值、中止、每次编译清理及 WSL 压缩各有记录，缺前提时给出责任人和解除条件。
- [ ] AC3（R3）：Issue 草稿含 `@nonghua123` 和可复现的 macOS 清单。发布后把 URL、确切验证 SHA 和结论挂到候选 PR。最终合并走 PR/Required CI/Merge Queue，并核对 `main` 合入 SHA；本地验收、Issue 发出和 macOS 通过是不同状态。

## Out of Scope

不在本机运行 macOS，不发布或签名、不改版本、不覆盖安装、不触碰生产配置或维护共享 WSL。允许对本次集成候选开 PR 和按仓库保护流程合入。

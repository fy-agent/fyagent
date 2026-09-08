# 行动计划与进度

## 顺序与退出条件

1. [x] 建立目标模式；记录根目录状态，从原分支建立隔离工作分支。
2. [x] 读取原任务、review 原提交与 0.4.4 差异；保留旧方案并写当前 PRD/design/implement。
3. [x] 解决主线合并冲突，保留原分支与可核验的同树来源，保持主线已交付行为。
4. [x] Native：复用 vault/SecretRef 选择账号，安全构造绑定，接通 Claude 应用与 Codex Change Plan/本机转发；补回归测试。
5. [x] Renderer：接当前 Managed Auth 入口，完成账号/模型选择与逐目标应用，去掉 legacy 动态挂载，补交互和 port 测试。
6. [x] 跨层集成：本机假上游和隔离目录证明请求协议、选择账号、凭据隔离、配置回读/恢复与失败；核对现有生命周期。
7. [x] 完成当前主机标准质量检查、生产浏览器检查与必要的性能检查；记录真实账号/Windows证据边界。
8. [x] 独立 full-scope check agent review 并修复；同步当前 spec，不保留错误的旧活跃路径。
9. 提交关闭动作：明确文件清单提交、归档、推送新分支并创建 PR；在最终 Git/PR 回执确认远端 SHA/PR/CI，原工作区保持原样。文档不提前填写尚未创建的 PR 编号。

## 实施分工

Native 实施者只负责 `src-tauri/**`、相关 native tests 与必要 backend spec；Renderer 实施者只负责 `src/**`、`tests/renderer/**`、`tests/browser/**`、必要 frontend spec。双方可各自解决负责范围的合并冲突并 stage 明确文件，但不提交、不 push、不操作另一个人的文件；接口先同步。主控拥有任务/方案、依赖安装、整合检查、提交与 PR。

## 验证命令

遵循仓库当前工具链和 `mise run` API，外层以 `rtk` 包装：

- `mise run system:check`，缺失依赖时使用当前 bootstrap 的已授权安装步骤，不升级锁文件。
- `mise run rust:test -- <相关过滤或测试目标>`；最终 `mise run check` 覆盖标准 TypeScript、lint、unit、Rust 与合同检查。
- `mise run test:browser`；涉及界面导航/动画时串行执行 `mise run test:performance`。
- 归档前使用 exact active-task exclusion 的预归档检查，之后校验有效 context 路径；具体命令按实时 task runner 合同。
- 所有测试输出记录摘要与必要错误，不保存秘密。最终相关改动后生成正式结果。

## 证据与 PR

`research/original-branch-review-20260908.md` 记录旧代码；`research/implementation-evidence-20260908.md` 由主控记录最终差异、检查、native/真实账号/Windows 分层结果和残余限制。若真实订阅或目标机器不可用，不冒充成功；完成所有可独立运行的实现和检查，PR 明确剩余外部验证，不合并或发布。

旧父任务/子任务的历史矩阵不自动变成本轮全部已验收；更新其引用/状态说明以避免旧入口被再次实施。工作提交先于归档/会话记录，推送与 PR 已由用户明确授权，不再重复询问。

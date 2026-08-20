---
type: audit
status: reviewed
updated: 2026-08-13
review_on: 2026-11-13
authority: https://github.com/fy-agent/fyagent/issues/21
source: git:9be29455a081d3ff0bc761465672727d09ffb3e6
evidence: code_audit + local_artifact_audit + remote_repository_audit
---

# FyAgent 仓库治理审计（2026-08-13）

## 结论

本审计以公开规范仓库 `fy-agent/fyagent` 的
`origin/main@9be29455a081d3ff0bc761465672727d09ffb3e6` 为固定基线，覆盖 Issue
[#21](https://github.com/fy-agent/fyagent/issues/21) 要求的本地标识、工作站路径、
高置信度凭据形状、大对象和协作治理。交付分支为
`codex/issue-21-repository-governance`；产品运行时代码、公共 API、持久化格式、依赖和
Release workflow 不在本次写集。

当前候选内容已把六处真实 Windows 用户目录引用替换为稳定的语义占位符，并保留相邻
SHA-256、来源角色、证据等级和审计结论。受控回归检查遍历所有 tracked Markdown，未再
发现具体用户目录，同时继续允许本地化占位符和 `example/profile/...`
这类中性示例。本次不重写 Git 历史，因此已经发布的旧提交仍可能保留原始
路径；这是明确保留的公开历史边界。

依赖为零的审计 helper 对固定基线树及全部可达历史完成了内存内 blob 扫描，过程没有把
匹配文本或候选值写入报告。命中仅来自仓库中已审阅的测试夹具和脱敏示例；没有发现需要
暂停发布并私下轮换的可信凭据。当前树和可达历史均没有达到 10 MiB 审阅阈值的 blob。

## 仓库与协作治理

| 项目        | 审计结果                                                                                                                                                    |
| ----------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 规范来源    | `fy-agent/fyagent`，公开组织仓库，不是 fork，默认分支为 `main`                                                                                              |
| 本 checkout | `origin` 的 fetch/push 均指向规范仓库；这是本 checkout 的状态，不是对贡献者 remote 名称的要求                                                               |
| 外部贡献者  | 个人 fork 通常为 `origin`；规范 FyAgent 仓库另作 fetch 来源；CC Switch 同步 remote 是独立、只读的维护角色                                                   |
| README      | 中、英、日三份入口同步补足当前范围、WorkBuddy 独立入口、React/Vite → Tauri IPC → Rust services → SQLite/目标工具配置/本地代理架构、首次 checkout 和证据分层 |
| 贡献流程    | 从规范仓库最新 `main` 建分支 → 聚焦 PR → 精确 PR head 的 `CI / Required` → squash merge；不直接或 force-push `main`                                         |
| CODEOWNERS  | 保留既有 owner 映射；文字改为如实说明它默认用于路由审阅，只有实时保护策略启用 Code Owner review 时才成为门禁                                                |

审计时 `main` 要求通过 PR、对管理员同样生效，并禁止 force push 和删除，但尚未要求状态
检查或人工批准。已批准的最小调整是在本 PR 精确 head 首次获得 GitHub Actions 所属的
`CI / Required` 后，仅把该聚合检查加入保护（`strict: false`）；批准数保持 0，Code
Owner review 保持非强制。更新后必须重新读取保护状态并核对，才允许 squash merge。

## 本地标识与工作站路径

- 六处具体 Windows profile 路径已分别改为
  `<local-submission-draft>`、`<local-vibekey-project-archive>`、
  `<local-vibekey-driver>` 和 `<local-fyagent-source-image>`。
- 账户 owner、CODEOWNERS、许可证署名以及正常 Git author/committer 信息属于公开归属，
  不作为“本地账户标识”删除。
- 回归合同只拒绝 `C:\Users\<具体名称>\...` 这类 workstation 泄露，不扩大为对所有绝对
  路径的禁令；系统目录、环境变量和明确 demo/占位符保持有效。

## 安全扫描与大对象

扫描器通过 Git 的 NUL 分隔接口枚举对象，以 batch 接口在进程内读取 blob；输出只包含
扫描器版本、模式、来源 tree OID、类别、安全路径、对象 OID、计数、大小和失败类别。
对象读取、解析或枚举不完整时失败关闭。合成测试同时覆盖 current tree、删除后仍可达的
历史 blob、无路径对象、二进制内容、异常路径和原始候选从 stdout、stderr、JSON 及异常
表面完全消失。

固定基线 tree OID 为 `ba1f69afe72226c371927ead11222775ff117305`：

| 范围                 |   对象 |   blob | 路径关联 | 已审阅形状命中 | `>= 10 MiB` |
| -------------------- | -----: | -----: | -------: | -------------: | ----------: |
| 固定基线树           |  1,718 |  1,718 |    1,732 |             17 |           0 |
| 全部 refs 的可达历史 | 33,965 | 15,561 |   15,561 |             63 |           0 |

基线树的 17 次形状命中为 OpenAI key 形状 15 次、AWS access-key 形状 2 次；历史的 63
次命中分别为 59 次和 4 次。逐文件语义复核确认它们都是测试夹具或脱敏示例。两种范围的
GitHub token 和 private-key header 形状均为 0。这里的“0”只适用于 helper v1 实际覆盖
的高置信度模式，不能推广为对任意秘密格式的完整证明。

当前树最大 blob 为已审阅的营销 PNG（1,718,652 bytes）；历史最大 blob 为已审阅的视觉
复核 ZIP（4,235,544 bytes）。`.gitattributes` 只把未来的
`tests/e2e/visual-baselines/**/*.png` 纳入 Git LFS；仓库没有通用的 tracked/history 大小
门禁。10 MiB 仅是本次审计阈值，不是新增产品或 CI 限制。

## 可复现命令与版本

```text
mise exec -- node scripts/audit/repository-governance-scan.mjs current --treeish <treeish>
mise exec -- node scripts/audit/repository-governance-scan.mjs history
mise run test:unit tests/repositoryGovernanceScan.test.ts tests/currentDocsContract.test.ts
gh api -H "X-GitHub-Api-Version: 2022-11-28" repos/fy-agent/fyagent
```

| 工具 / 接口                | 版本                 |
| -------------------------- | -------------------- |
| repository-governance-scan | 1                    |
| Git                        | 2.55.0.windows.3     |
| GitHub CLI                 | 2.97.0               |
| Node.js（mise）            | 24.19.0              |
| mise                       | 2026.8.2 windows-x64 |
| GitHub REST API            | 2022-11-28           |

2026-08-13 的 GitHub `security_and_analysis` 响应显示 Secret Scanning、non-provider
patterns、validity checks、push protection 和 Dependabot security updates 均为 disabled。
Secret Scanning alerts 因功能未启用而不可作为证据，因此本审计不声称“GitHub alerts 为
零”，也不把本地 helper 等同于 GitHub Secret Scanning 或专用凭据扫描产品。

## 验证与剩余边界

- 聚焦合同：`tests/currentDocsContract.test.ts` 14/14、
  `tests/repositoryGovernanceScan.test.ts` 6/6，共 20/20 通过。
- 当前 Windows checkout 已在 Visual Studio Developer PowerShell 环境完成
  `mise run system:check`，确认 Git、WebView2 与 `cl.exe` 工具链可用。
- 当前候选于 2026-08-13 完成 `mise run release:check` 与完整 `mise run check`；
  本地发布合同和项目门禁均通过。该结果只约束本地候选，远程 `CI / Required`
  仍必须在精确远端提交上独立成功，不能由本地结果替代。
- 本审计不证明原生窗口 HIL、安装器生命周期、签名、公证或正式 Release。最终 PR head、
  squash merge SHA、Issue 关闭状态和合并后精确 `main` push CI 由 GitHub PR/Actions
  记录承担，不在文档中预写尚未发生的结果。
- 不改写历史、不新增通用 secret/size CI 产品，也不删除正常归属信息。若未来出现可信
  secret 或保护策略竞态，应停止公开发布，私下完成轮换/协调；合并后的内容回退使用普通
  revert PR，禁止 force-push `main`。

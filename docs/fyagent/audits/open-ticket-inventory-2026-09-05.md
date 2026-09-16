---
type: inventory-draft
status: draft
updated: 2026-09-05
scanned_at: 2026-09-05T18:51:34Z
previous_scan: 2026-08-17T20:45:50Z
authority: inventory-only
source: gh repo/issue/pr/release + GraphQL discussions + local git (read-only)
---

# 开放票、版本与分支盘点草稿（2026-09-05）

> **这是盘点草稿，不是方案，不是验收。**
>
> 已开盘点，尚未提交方案。不关闭任何 issue / discussion / PR，
> 不把进行中的工作写成已完成或已验收，不对外承诺工期或范围。
> 请勿合并进 `main`，除非仓库主人明确要求。
>
> 上次盘点 PR [#109](https://github.com/fy-agent/fyagent/pull/109) 已于
> 2026-08-23 关闭且未合入；远程分支 `cursor/open-ticket-inventory-15a7` 已不存在。
> 本文是重扫，不是把 #109 写成已落地。
>
> 并行线：[#35](https://github.com/fy-agent/fyagent/issues/35) 仍 OPEN。
> 本盘点不停它，也不把它写成完成。
>
> [#102](https://github.com/fy-agent/fyagent/issues/102) 上次给定的实测时间是
> **2026-08-19 10:00**（不是 2026-08-18）。本次扫描**未见**该时点的正式矩阵回写；
> 也不把 2026-09-05 写成 #102 实测日。

## 1. 和 2026-08-17 盘点比，先看变化

| 项 | 2026-08-17 | 2026-09-05 | 事实 |
| --- | --- | --- | --- |
| 可见公开仓 | 6 | 9 | 个人仓 +1、组织仓 +2；`fyagent-Original` 现解析为 `NongHua123/fyagent` |
| `fy-agent/fyagent` 开放 issue | 74 | 68 | 关 8 张旧票 + 后开后关的 #110；新开仍开放 #141、#179 |
| 同仓开放 discussion | 5 | 6 | 新增 #133 |
| 同仓开放 PR | 1（#108） | 1（#172） | #108 已关；#172 开放且与 `main` 冲突 |
| 树内版本 / 最新 Release | 当时未盘版本路径 | Cargo `0.4.3` / Release `v0.4.3` | `main` 比 `v0.4.3` 多 79 个提交 |
| #35 | OPEN，并行线 | 仍 OPEN | #145 已合 SecretRef 核心切片；票面写明未完成验收 |
| #102 | OPEN，矩阵未收齐 | 仍 OPEN | 有 0.4.0 单人评论和 2026-09-05 技能收录评论；未见整票验收 |
| #105 / #107 | OPEN | 均已关闭 | #105：2026-08-20；#107：2026-08-24 |
| 上次盘点 PR | draft #109 | CLOSED，未合并 | 2026-08-23 关 |

## 2. 扫描范围与可见仓库

扫描时刻（UTC）：`2026-09-05T18:51:34Z`。
凭据：当前 `gh` 为 GitHub App `cursor`。
`orgs/fy-agent.total_private_repos` 仍为 `null`。本盘点不能证明没有私有仓。

`users/NongHua123.public_repos=5`，`orgs/fy-agent.public_repos=4`，与下表 9 仓一致。

| 所有者 | 仓库 | 可见性 | Fork | 相对上次 | 开放 issue | 开放 discussion |
| --- | --- | --- | --- | --- | ---: | ---: |
| NongHua123 | [fyagent](https://github.com/NongHua123/fyagent) | public | 是（parent=`fy-agent/fyagent`） | 原 `fyagent-Original` 现解析到此名 | 2 | 2 |
| NongHua123 | [fde-weekly-report-course-candidate-20260823](https://github.com/NongHua123/fde-weekly-report-course-candidate-20260823) | public | 否 | 新出现 | 0 | 0（未启用） |
| NongHua123 | [harper](https://github.com/NongHua123/harper) | public | 是 | 仍在 | 0（Issues 已关闭） | 0（未启用） |
| NongHua123 | [WeChat-Agent-Workflows](https://github.com/NongHua123/WeChat-Agent-Workflows) | public | 否 | 仍在 | 0 | 0（未启用） |
| NongHua123 | [wechat-chatgpt](https://github.com/NongHua123/wechat-chatgpt) | public | 否 | 仍在 | 0 | 0（未启用） |
| fy-agent | [fyagent](https://github.com/fy-agent/fyagent) | public | 否 | 仍在 | 68 | 6 |
| fy-agent | [humanize-chinese-writing](https://github.com/fy-agent/humanize-chinese-writing) | public | 否 | 仍在 | 0 | 0（未启用） |
| fy-agent | [FY1111](https://github.com/fy-agent/FY1111) | public | 否 | 新出现 | 0 | 0（未启用） |
| fy-agent | [ai-work-pra-offl](https://github.com/fy-agent/ai-work-pra-offl) | public | 否 | 新出现 | 0 | 0（未启用） |

`NongHua123/fyagent-Original` 的 `gh repo view` 现在返回 `nameWithOwner=NongHua123/fyagent`（重定向/更名，不是第二份独立仓）。

开放 PR：`fy-agent/fyagent` 1 张；其余可见仓 0 张。

## 3. 版本迭代路径（`fy-agent/fyagent`）

当前 checkout / `origin/main`：`967d6aae`（2026-09-04，Merge PR #178）。
树内 Cargo 版本：`0.4.3`。`CHANGELOG.md` 最新正式节是 `[0.4.3] - 2026-09-01`，**没有 Unreleased 节**。

### 3.1 标签、Release、树

| 版本 | 标签日 | GitHub Release | 备注（票面/文档事实） |
| --- | --- | --- | --- |
| v0.3.0 | 2026-08-08 | 有（2026-08-08） | 公开 Release 存在 |
| v0.3.1 | 2026-08-10 | 无对应 Release 页 | 有 tag；CHANGELOG 有节 |
| v0.3.2 | 2026-08-11 | 无 | 有 tag；CHANGELOG 有节 |
| v0.3.3 | 2026-08-12 | 无 | 有 tag；CHANGELOG 有节 |
| v0.3.4 | 2026-08-12 | 有（2026-08-11） | 上次盘点前后的已安装实测基线之一 |
| v0.4.0 | 2026-08-20 | 有（2026-08-20） | V2 桌面壳；#102 / #138 UAT 用过此版 |
| v0.4.1 | 2026-08-20 | **无** | CHANGELOG 写明：Release 跑公证等待失败，从未发布；tag 仍在原提交 |
| v0.4.2 | 2026-08-21 | 有（2026-08-20） | 补公证/DMG；#131 macOS UAT 用此版 |
| v0.4.3 | 2026-09-01 | 有，且标 Latest（2026-09-01） | 当前公开 Latest；SecretRef / Change Plan / 桌面生命周期收口写入发布说明 |
| `main` @ 967d6aae | 2026-09-04 | 无新 tag | 比 `v0.4.3` **超前 79 个提交**。未标 0.4.4 / 0.5.0 |

公开 Release 页现有：`v0.3.0`、`v0.3.4`、`v0.4.0`、`v0.4.2`、`v0.4.3`。
`v0.4.3` 发布说明列 7 个附件：macOS DMG、Windows x64/arm64 setup、三份元数据、attestation。

### 3.2 `v0.4.3` 之后已经进 `main`、尚未打新版本的合并

这些已合入 `main`，**不等于**已发正式版：

| 合入日 | PR | 标题（原文） |
| --- | --- | --- |
| 2026-09-01 | #173 | feat(macos): unify Agent installs and privileged system commits |
| 2026-09-02 | #174 | docs: refresh README screenshots and gallery layout |
| 2026-09-02 | #175 | feat: finalize desktop lifecycle, V2 reliability, and Trellis specs |
| 2026-09-03 | #176 | feat: unify managed agent authentication and tooling flows |
| 2026-09-03 | #177 | ci: move CodeQL off the pull request hot path |
| 2026-09-04 | #178 | fix: stabilize macOS launch, task termination, and Codex auth |

`dev/laiyongjie` 在此之后还有未进 `main` 的 2026-09-05 前端 round-3 / round-4 提交（见第 4 节）。

### 3.3 个人 fork 上的 v0.5.0 讨论

`NongHua123/fyagent` 有两张标题相同的开放 Ideas：

- [#3](https://github.com/NongHua123/fyagent/discussions/3)（2026-08-20 18:22）
- [#4](https://github.com/NongHua123/fyagent/discussions/4)（2026-08-20 19:07）

标题均为「v0.5.0 迭代规划讨论：V2 功能补齐与迁移收尾」，各 0 条评论。
规范仓 `fy-agent/fyagent` **没有**同名开放 discussion，也没有 `0.5.0` tag / CHANGELOG 节。
这是 fork 上的重复讨论草稿，不是已宣布的下一正式版。

## 4. 分支与合并

远端可见分支：`main`、`dev/laiyongjie`、`feat/grok-first-class-iteration`、`demo/shurufa`、`star-history`。

| 分支 | 相对 `origin/main` | 开放 PR | 一句话现状 |
| --- | --- | --- | --- |
| `main` | 自身 | — | 默认分支；最新合并 #178（2026-09-04） |
| `dev/laiyongjie` | main 多 4 / 本分支多 29 | 无针对这 29 提交的开放 PR | 2026-09-05 前端 experience round-3/4、motion/surfaces 等仍只在此分支 |
| `feat/grok-first-class-iteration` | main 多 125 / 本分支多 1 | [#172](https://github.com/fy-agent/fyagent/pull/172) OPEN | `mergeable=CONFLICTING`，`DIRTY`；Commit Convention 检查 FAILURE；2026-08-30 后未更新 |
| `demo/shurufa` | 分叉（main 165 / 本分支 19） | 无 | 未并入 main |
| `star-history` | 分叉极大（main 2827 / 本分支 1） | 无 | 与当前 main 历史不对齐；star-history 相关改动已另走 #156/#157/#163/#164 合入 |

2026-08-17 之后合入 `main` 的 PR 至少 40+ 张，主路径是反复 `dev/laiyongjie -> main`，另有 Change Plan / SecretRef 抢救链（#145–#149）、前端 V3（#158/#159）、发布 0.4.0–0.4.2（#116–#122）。

关闭且未合并、与 UAT/恢复相关的 PR（不是本刀关的）：

- #131 / #138：installed-app UAT 文档 PR，结论 NO-GO；发现迁到 #141 后关闭不合并
- #132 / #134 / #136 / #137 / #139 / #143 等 draft 恢复 PR：2026-08-24 关闭；后续由 #145–#149 等合入切片替代
- #109：上次盘点草稿，关闭未合并

## 5. `fy-agent/fyagent` 开放 issue（68）

标注只描述票面，不是关票建议。

GitHub assignee 非空仍是 5 张，全部 `python-rust`：#22、#34、#47、#101、#102。
`status:queued`：22 张（#22、#25–#29、#32、#34、#45、#47、#48、#50–#53、#61、#67、#68、#70–#72、#101）。
无标签：#102、#179。
P0 仍开放 17 张；P1 40；P2 8；bug 1（#141）。

### 5.1 上次之后新开、仍开放

| # | 标题 | 作者 | 更新 | 标注 | 一句话现状 |
| ---: | --- | --- | --- | --- | --- |
| [#141](https://github.com/fy-agent/fyagent/issues/141) | [UAT] 汇总 macOS 0.4.2 / Windows 0.4.0 NO-GO 验收发现与复验计划 | python-rust | 2026-08-30 | 缺 owner；承接已关 PR | 正文写 #131/#138 为历史已装版本 NO-GO，不自动证明当前 main。#131/#138 已关不合并。2026-08-30 回写：本迭代若改 Grok 草稿则复验 B7，B9 不在范围。未见整票复验完成。 |
| [#179](https://github.com/fy-agent/fyagent/issues/179) | OrcaRouter provider support for FyAgent | kuswardhanietidims-svg | 2026-09-05 | 缺 owner；缺产品标签 | 外部贡献意向：把 OrcaRouter 当作可选 OpenAI-compatible provider。0 评论。不是验收票。 |

### 5.2 仍开放的组级 / 并行 / 实测票

| # | 标题 | 指派 | 更新 | 标注 | 一句话现状 |
| ---: | --- | --- | --- | --- | --- |
| [#35](https://github.com/fy-agent/fyagent/issues/35) | [G2-02] 建立凭据引用与本机/硬件可插拔后端 | — | 2026-08-24 | 正在做/并行线；已有结论未关 | #145 已合 SecretRef 核心，票面 2026-08-24 写明生产未注册、缺 consumer/HIL，保持 OPEN。本盘点不停它。 |
| [#63](https://github.com/fy-agent/fyagent/issues/63) | [G4-09] Codex Provider 新建/编辑/切换进 Change Plan | — | 2026-08-24 | 部分完成仍开放 | 仍 OPEN。后续有 #148 等 salvage 合入，整票验收未关。 |
| [#101](https://github.com/fy-agent/fyagent/issues/101) | [PRD] 首次目标选择、Agent 目录与既有配置接管 | python-rust | 2026-08-23 | 父子重叠；status:queued | 仍 OPEN。优先级现为 P1 + queued。 |
| [#102](https://github.com/fy-agent/fyagent/issues/102) | [W33][全员] 一期候选 Agent 实测矩阵 | python-rust | 2026-09-04 | 票面截止已过；正在做 | 正文截止 2026-08-13 已过。给定实测时点 2026-08-19 10:00 未见正式矩阵回写。C8 为 2026-08-20 诸葛愉嘉 0.4.0 单人记录。C9 为 2026-09-04 liyangbing 技能收录核验，不是 6 人矩阵收口。未验收、未关。 |
| [#22](https://github.com/fy-agent/fyagent/issues/22) / [#34](https://github.com/fy-agent/fyagent/issues/34) / [#47](https://github.com/fy-agent/fyagent/issues/47) | 目录 / 首次目标 / 保持并回读 | python-rust | 2026-08-23 | 父子重叠；queued | 仍 OPEN，现多为 P1 + queued。 |

### 5.3 其余仍开放的 G 系列（按组）

这些票上次已在册，本次仍 OPEN。多数无 GitHub assignee。`updatedAt` 停在 2026-08-12 的，票面无新回写；2026-08-23/24/30 有回写的已在标签或评论上动过，不等于验收。

| 组 | 仍开放 | 相对上次关掉 |
| --- | --- | --- |
| G1 目录/安装 | #22–#34（缺已关号） | 无新增关张（#20 是维护票，见下） |
| G2 接入/凭据 | #34–#45 | 无 |
| G3 首成/健康 | #47、#48、#50–#54 | 无 |
| G4 Change Plan | #55–#66 | 无 |
| G5 发布/Pack/Profile | #67、#68、#70–#78 | #69、#79、#80 已关（2026-08-23） |
| G6 主张/素材 | #81–#87、#89、#90、#92 | #88、#91 已关（2026-08-23）；#93 上次扫描日已关 |
| 维护 | — | #20 已关（2026-08-18） |

完整编号（68）：#22–#34、#35–#45、#47、#48、#50–#92 中仍开的上述集合，外加 #101、#102、#141、#179。

G1–G6 单票一句话：除第 5.2 节点名的票外，其余仍是「有验收清单或复核/规划评论；无 GitHub assignee；未见整票关闭」。#45 上次标缺验收标题，本次未重读全文，不改该观察。#41/#55–#60/#66 在 2026-08-24 有 UCP salvage 评论或关联已合 PR，票仍 OPEN。

### 5.4 上次开放、本次已关（只记录，不是本刀关的）

| # | 关日 | 标题 |
| ---: | --- | --- |
| #20 | 2026-08-18 | 停止官方 Linux 平台支持 |
| #69 | 2026-08-23 | macOS Developer ID 签名和公证 |
| #79 | 2026-08-23 | Portable Context 纠正/忘记/授权 |
| #80 | 2026-08-23 | Portable Context 披露对象 |
| #88 | 2026-08-23 | 禁止伪产品界面 |
| #91 | 2026-08-23 | v4 控制面原型评审后冻结品牌 |
| #105 | 2026-08-20 | Skills 发现页全量加载缓慢 |
| #107 | 2026-08-24 | Grok Official 登录文案点名 grok login |
| #110 | 2026-08-21 | 一键安装只保证来源（上次盘点后新开又关） |

#107 关闭 ≠ discussion #106 关闭。#106 仍开放，并在 2026-08-30 写下 Grok 迭代范围。

## 6. 开放 discussion

### 6.1 `fy-agent/fyagent`（6）

| # | 标题 | 分类 | 更新 | 评论 | 一句话现状 |
| ---: | --- | --- | --- | ---: | --- |
| [#106](https://github.com/fy-agent/fyagent/discussions/106) | Grok 已经在名单里了，把登录收完… | Ideas | 2026-08-30 | 5 | 产品意图仍挂在本讨论。2026-08-30 回写：本迭代做三条登录拆开（#43）+ SuperGrok 投到 Claude/Codex/WorkBuddy（#42）；不做安装升级、额度看板、总门卫。写明须 William 在 Windows 和 Mac mini 上 HIL，合同/CI 不能替代。不关 #42/#43 整票。对应 PR #172 仍冲突未合。 |
| [#133](https://github.com/fy-agent/fyagent/discussions/133) | 下一步：做 AI 助手的“总门卫” | Ideas | 2026-08-23 | 0 | 正文自写：方向保留，现在不急着开发。点名工程单 #35/#45/#66/#77。#106 明确本迭代不做总门卫。 |
| [#94](https://github.com/fy-agent/fyagent/discussions/94) | Welcome to FyAgent Discussions | Announcements | 2026-08-12 | 0 | 社区种子 |
| [#95](https://github.com/fy-agent/fyagent/discussions/95) | How to ask a question… | Q&A | 2026-08-12 | 0 | 种子；isAnswered=false |
| [#96](https://github.com/fy-agent/fyagent/discussions/96) | What should your AI Worker make easier next? | Ideas | 2026-08-12 | 0 | 社区种子 |
| [#97](https://github.com/fy-agent/fyagent/discussions/97) | Show how you make AI your own | Show and tell | 2026-08-12 | 0 | 社区种子 |

### 6.2 `NongHua123/fyagent`（2）

| # | 标题 | 更新 | 评论 | 标注 |
| ---: | --- | --- | ---: | --- |
| [#3](https://github.com/NongHua123/fyagent/discussions/3) | v0.5.0 迭代规划讨论：V2 功能补齐与迁移收尾 | 2026-08-20 | 0 | 与 #4 标题重复；在 fork 上 |
| [#4](https://github.com/NongHua123/fyagent/discussions/4) | 同上 | 2026-08-20 | 0 | 同上 |

### 6.3 个人 fork 开放 issue（2）

| # | 标题 | 更新 | 现状 |
| ---: | --- | --- | --- |
| [#1](https://github.com/NongHua123/fyagent/issues/1) | [harness] 把 710 行的 workflow.md 拆开 | 2026-08-20 | 0 评论；在 fork，不在规范仓 |
| [#2](https://github.com/NongHua123/fyagent/issues/2) | [harness] Trellis 脚本加备用动力 | 2026-08-20 | 0 评论；在 fork |

## 7. 分类摘录（仍不是方案）

### 7.1 明显重复或成对

- #101 ↔ #22 / #34 / #47：组级 PRD 与子票仍同时开放。
- discussion #106 ↔ issue #42 / #43，以及开放 PR #172：Grok 登录/投放。#107 已关，讨论未关。
- #141 ← 已关 PR #131 / #138：历史 UAT NO-GO 汇总。
- `NongHua123/fyagent` discussion #3 与 #4：标题相同。
- #102 C9 技能收录核验与 #102 原定「6 人实测矩阵」主题不一致；只记录，不改票。

### 7.2 票面日期已过 / 版本已旧

- #102 正文截止 2026-08-13 已过。给定 2026-08-19 10:00 未见正式矩阵回写。
- #141 验证的是 0.4.2 macOS / 0.4.0 Windows 已装包；正文自己写必须在最新 main 复验。当前 Latest 已是 0.4.3，main 又超前 79 提交。
- #172 相对 main 落后 125 提交，2026-08-30 后无更新。

未把「几天没评论」的 G 系列标成过期。

### 7.3 缺验收 / 缺 owner

- 新票 #179：无验收勾选被维护者确认，也无产品标签。
- 讨论帖 #94–#97、#133、fork #3/#4：不是带验收清单的交付票。
- 63 / 68 张规范仓开放 issue 无 GitHub assignee（与上次 69/74 同类）。

### 7.4 正在做（未写成已完成）

- #35 并行线，仍 OPEN。
- `dev/laiyongjie` 有未合入 main 的 2026-09-05 前端提交。
- #172 开放但冲突。
- #102 仍在收评论，未关。
- #106 写明本迭代要做 Grok 登录拆开与 SuperGrok 投放，并要求 William HIL；未见 HIL 完成回写。

### 7.5 已有结论未关

- #35：切片已合，整票按票面保持 OPEN。
- #63 / #41 / #55–#60 / #66：UCP salvage 有合入记录，整票仍 OPEN。
- #141：历史 NO-GO 已迁入，复验未关。
- #133：方向「现在不急着开发」，讨论仍开。
- #106：范围已写，讨论仍开。

## 8. 待后续写方案的候选问题（仅候选）

本刀不排序、不选方案。

| 候选簇 | 主要开放物 | 相对上次的事实变化 |
| --- | --- | --- |
| 版本/发布路径 | 无对应 issue；CHANGELOG + Release | Latest=0.4.3；main 超前；0.4.1 有 tag 无 Release |
| `dev/laiyongjie` 未合入前端 | 无开放 PR | 29 提交只在开发分支 |
| Grok 一等公民登录/投放 | #106、#42、#43、PR #172 | #107 已关；#172 冲突 |
| 凭据 / Change Plan 主链 | #35、#41、#55–#66 | 有 salvage 合入；#35/#63 未关 |
| 首次目标 / 目录 / 回读 | #101、#22、#34、#47 | 现多标 queued |
| 历史 UAT 复验 | #141 | 新票；针对旧安装包 |
| 全员实测矩阵 | #102 | 仍开放；无 2026-08-19 10:00 正式回写 |
| 外部 provider 意向 | #179 | 新票，0 评论 |
| 总门卫方向 | #133 | 新讨论；自写暂不开发 |
| 主张/素材 G6 | #81–#92 仍开部分集合 | #88/#91 已关 |
| 发布身份/Windows 签名 | #67、#68、#70–#72 | queued；#69 macOS 公证票已关 |
| fork 上的 Trellis harness / v0.5.0 讨论 | NongHua123/fyagent #1 #2 #3 #4 | 不在规范仓 |

#102 评论里仍未见单独成票的项（最大化/还原、UI 对比度等）本次未再核代码，不升级结论。

## 9. 本刀不做的事

- 不提交 PRD / 技术设计 / 详细设计
- 不代关 issue / discussion / PR
- 不推、不合并 `main`
- 不改 #102 的 2026-08-19 10:00 记录
- 不停 #35
- 不把 #145、#178、#105、#107、0.4.3 Release 写成「全仓问题已解决」

## 10. 完成状态

**已开盘点，尚未提交方案。**

交付物：可见仓清单、版本路径、分支合并、每仓开放 issue/discussion、相对 2026-08-17 的开关差、后续写方案候选簇。
未交付：方案结论、关票、验收、对外承诺。

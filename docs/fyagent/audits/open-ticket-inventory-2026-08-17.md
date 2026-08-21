---
type: inventory-draft
status: draft
updated: 2026-08-17
scanned_at: 2026-08-17T20:45:50Z
authority: inventory-only
source: gh issue list + GraphQL discussions (read-only)
---

# 开放 Issue / Discussion 盘点草稿（2026-08-17）

> **这是盘点草稿，不是方案，不是验收。**
>
> 已开盘点，尚未提交方案。本文件不关闭任何 issue / discussion / PR，
> 不把进行中的工作写成已完成或已验收，不对外承诺工期或范围。
> 请勿合并进 `main`，除非仓库主人明确要求。
>
> 并行线：[#35](https://github.com/fy-agent/fyagent/issues/35) 继续独立推进；
> 本盘点不是 #35 的完成证明。
>
> [#102](https://github.com/fy-agent/fyagent/issues/102) 的实测时间是
> **2026-08-19 10:00**（不是 2026-08-18，也不是 2026-08-17 实测）。

## 1. 扫描范围与可见性

扫描时刻（UTC）：`2026-08-17T20:45:50Z`。
凭据：当前 `gh` 以 GitHub App `cursor` 登录。
`orgs/fy-agent` 的 `total_private_repos` 返回 `null`（无组织管理员字段）。
`users/NongHua123/orgs` 返回空数组。

当前凭据能列出的仓库共 **6** 个，全部公开、未归档：

| 所有者 | 仓库 | 可见性 | Fork | Issues | Discussions | 开放 issue | 开放 discussion |
| --- | --- | --- | --- | --- | --- | ---: | ---: |
| NongHua123 | [fyagent-Original](https://github.com/NongHua123/fyagent-Original) | public | 是 | 已关闭 | 未启用 | 0 | 0 |
| NongHua123 | [harper](https://github.com/NongHua123/harper) | public | 是 | 已关闭 | 未启用 | 0 | 0 |
| NongHua123 | [WeChat-Agent-Workflows](https://github.com/NongHua123/WeChat-Agent-Workflows) | public | 否 | 启用 | 未启用 | 0 | 0 |
| NongHua123 | [wechat-chatgpt](https://github.com/NongHua123/wechat-chatgpt) | public | 否 | 启用 | 未启用 | 0 | 0 |
| fy-agent | [fyagent](https://github.com/fy-agent/fyagent) | public | 否 | 启用 | 启用 | 74 | 5 |
| fy-agent | [humanize-chinese-writing](https://github.com/fy-agent/humanize-chinese-writing) | public | 否 | 启用 | 未启用 | 0 | 0 |

公开计数核对：`users/NongHua123.public_repos=4`，`orgs/fy-agent.public_repos=2`，与上表一致。

未列入、但评论里出现过的名字：`fy-agent/novelist`。
当前凭据 `gh repo view fy-agent/novelist` 返回仓库不存在或不可见。
本盘点**不能**证明组织下没有私有仓，只能证明：**这把钥匙看不到更多仓**。

本刀只读。没有关闭、没有评论、没有改票面日期、没有推 `main`。

## 2. 各仓数量

| 仓库 | 开放 issue | 开放 discussion | 开放 PR（只列数量，本刀不审 PR） |
| --- | ---: | ---: | ---: |
| NongHua123/fyagent-Original | 0（Issues 已关闭） | 0（未启用） | 0 |
| NongHua123/harper | 0（Issues 已关闭） | 0（未启用） | 0 |
| NongHua123/WeChat-Agent-Workflows | 0 | 0（未启用） | 0 |
| NongHua123/wechat-chatgpt | 0 | 0（未启用） | 0 |
| fy-agent/fyagent | 74 | 5 | 1（[#108](https://github.com/fy-agent/fyagent/pull/108)） |
| fy-agent/humanize-chinese-writing | 0 | 0（未启用） | 0 |
| **合计（可见仓）** | **74** | **5** | **1** |

NongHua123 个人仓开放 issue / discussion 均为 0。
开放票全部落在 `fy-agent/fyagent`。

## 3. 标注说明

下表「标注」只描述票面事实，不是处理建议，也不等于要关票。

| 标注 | 用法 |
| --- | --- |
| 缺 owner | GitHub `assignees` 为空。评论里出现的人名不算 GitHub owner。 |
| 缺验收标准 | 正文没有「验收」标题，也没有勾选清单。 |
| 父子重叠 | #101 自称组级入口，并点名 #22 / #34 / #47；职责交叉，不是抄错标题。 |
| 与讨论成对 | 同一主题同时存在 discussion 与 issue。 |
| 从 #102 拆出 | 票面写明反馈来自 #102。 |
| 票面截止已过 | 正文写了早于扫描日的截止，票仍开放。 |
| 日切片日期已过 | 评论里的「今日执行切片」日期早于扫描日。 |
| 已指派/有近期切片 | GitHub 有 assignee，或 2026-08-13 仍有执行切片回写；不等于已完成。 |
| 部分完成仍开放 | 票面自己写明子范围完成、整票继续 OPEN。 |
| 正在做 | 有开放 PR、明确阻塞回写、或票面写明未跑完的实测。 |
| 正在做/并行线 | 仅 #35。本盘点不并入、不停它。 |
| 已有结论未关 | 评论已写决策、规划完成或子范围完成，整票仍 OPEN。 |
| Future | 标签 `priority:Future`。 |
| 社区种子 | 2026-08-12 启动的讨论区置顶/分类帖，不是交付票。 |

GitHub assignee 非空的开放 issue 只有 5 张，全部是 `python-rust`：
#22、#34、#47、#101、#102。其余 69 张无 GitHub assignee。

正文缺验收标题且无勾选清单的开放 issue：只有 **#45**。
#107 有「验收」段，无勾选框。其余 G 系列与 #20 / #101 / #102 / #105 正文都有验收或勾选。

## 4. `fy-agent/fyagent` 开放 issue

按编号升序。更新日为 GitHub `updatedAt` 的 UTC 日期。

| # | 标题 | 作者 | 标签 | 指派 | 更新 | 评论 | 标注 | 一句话现状 |
| ---: | --- | --- | --- | --- | --- | ---: | --- | --- |
| [#20](https://github.com/fy-agent/fyagent/issues/20) | [维护计划] 停止官方 Linux 平台支持并逐步移除相关代码 | python-rust | — | — | 2026-08-12 | 0 | 缺 owner | 维护计划已写分阶段清理清单；无评论、无 GitHub assignee，未见执行回写。 |
| [#22](https://github.com/fy-agent/fyagent/issues/22) | [G1-01] 建立 Agent 目录事实合同并驱动可用选项 | NongHua123 | enhancement,priority:P0 | python-rust | 2026-08-13 | 5 | 父子重叠；日切片日期已过；已指派/有近期切片 | 目录事实合同；有调研与 2026-08-13 W33 切片；GitHub assignee=python-rust；未写完成。 |
| [#23](https://github.com/fy-agent/fyagent/issues/23) | [G1-02] 按使用目标搜索和筛选 Agent | NongHua123 | enhancement,priority:P1 | — | 2026-08-12 | 1 | 缺 owner | 有交付验收清单与 2026-08-12 复核评论；无 GitHub assignee。 |
| [#24](https://github.com/fy-agent/fyagent/issues/24) | [G1-03] 一次比较最多三个 Agent，并解释推荐理由 | NongHua123 | enhancement,priority:P1 | — | 2026-08-12 | 1 | 缺 owner | 有交付验收清单与 2026-08-12 复核评论；无 GitHub assignee。 |
| [#25](https://github.com/fy-agent/fyagent/issues/25) | [G1-04] 展示官方来源、许可和镜像来路 | NongHua123 | enhancement,priority:P0 | — | 2026-08-13 | 5 | 缺 owner | 有调研补充；2026-08-13 回写「仅赖永杰 Git-only」；无 GitHub assignee。 |
| [#26](https://github.com/fy-agent/fyagent/issues/26) | [G1-05] 安装前验证包的 hash、签名和撤回状态 | NongHua123 | enhancement,priority:P0 | — | 2026-08-13 | 4 | 缺 owner | 有调研补充；2026-08-13 回写「仅赖永杰 Git-only」；无 GitHub assignee。 |
| [#27](https://github.com/fy-agent/fyagent/issues/27) | [G1-06] 在下载前完成环境预检 | NongHua123 | enhancement,priority:P0 | — | 2026-08-13 | 4 | 缺 owner | 有调研补充；2026-08-13 回写「仅赖永杰 Git-only」；无 GitHub assignee。 |
| [#28](https://github.com/fy-agent/fyagent/issues/28) | [G1-07] 生成不可静默变化的安装计划 | NongHua123 | enhancement,priority:P0 | — | 2026-08-13 | 3 | 缺 owner；已有结论未关 | 评论写组级决策 PRD 已完成（并写明非代码、非验收）；2026-08-13 回写 Git-only；无 GitHub assignee。 |
| [#29](https://github.com/fy-agent/fyagent/issues/29) | [G1-08] 用普通用户主程序和窄权限 Helper 执行安装 | NongHua123 | enhancement,priority:P0 | — | 2026-08-12 | 1 | 缺 owner | 有交付验收清单与 2026-08-12 复核评论；无 GitHub assignee。 |
| [#30](https://github.com/fy-agent/fyagent/issues/30) | [G1-09] 支持多 Agent 有序安装、断点继续和逐项结果 | NongHua123 | enhancement,priority:P1 | — | 2026-08-12 | 0 | 缺 owner | 有交付验收清单，无评论；无 GitHub assignee。 |
| [#31](https://github.com/fy-agent/fyagent/issues/31) | [G1-10] 识别多份安装，并让版本与更新策略可控 | NongHua123 | enhancement,priority:P1 | — | 2026-08-12 | 1 | 缺 owner | 有调研补充；无 GitHub assignee。 |
| [#32](https://github.com/fy-agent/fyagent/issues/32) | [G1-11] 安装后必须通过真实健康探测 | NongHua123 | enhancement,priority:P0 | — | 2026-08-12 | 1 | 缺 owner | 有交付验收清单与 2026-08-12 复核评论；无 GitHub assignee。 |
| [#33](https://github.com/fy-agent/fyagent/issues/33) | [G1-12] 安全卸载 Agent，不误删工作区和用户数据 | NongHua123 | enhancement,priority:P1 | — | 2026-08-12 | 0 | 缺 owner | 有交付验收清单，无评论；无 GitHub assignee。 |
| [#34](https://github.com/fy-agent/fyagent/issues/34) | [G2-01] 首次进入先问目标，再按目录给出 Agent 与接入方式 | NongHua123 | enhancement,priority:P0 | python-rust | 2026-08-13 | 3 | 父子重叠；已指派/有近期切片 | 正文写明吸收已关的 #46；指向组级 PRD #101；GitHub assignee=python-rust。 |
| [#35](https://github.com/fy-agent/fyagent/issues/35) | [G2-02] 建立凭据引用与本机/硬件可插拔后端 | NongHua123 | enhancement,priority:P0 | — | 2026-08-13 | 2 | 缺 owner；正在做/并行线 | 并行线。2026-08-12 收窄为 secretRef/可插拔后端；#63 写 DESIGN_FREEZE=PENDING；无 GitHub assignee。本盘点不处理、不停它。 |
| [#36](https://github.com/fy-agent/fyagent/issues/36) | [G2-03] 迁移旧凭据，并把外部工具的明文边界讲清楚 | NongHua123 | enhancement,priority:P1 | — | 2026-08-12 | 0 | 缺 owner | 有交付验收清单，无评论；无 GitHub assignee。 |
| [#37](https://github.com/fy-agent/fyagent/issues/37) | [G2-04] 接入阿里开放 API，并完成目标 Agent 真实验证 | NongHua123 | enhancement,priority:P1 | — | 2026-08-12 | 1 | 缺 owner | 有 2026-08-12 复核评论；无 GitHub assignee。 |
| [#38](https://github.com/fy-agent/fyagent/issues/38) | [G2-05] 接入腾讯开放 API，并完成目标 Agent 真实验证 | NongHua123 | enhancement,priority:P1 | — | 2026-08-12 | 1 | 缺 owner | 有 2026-08-12 复核评论；无 GitHub assignee。 |
| [#39](https://github.com/fy-agent/fyagent/issues/39) | [G2-06] 接入字节跳动开放 API，并完成目标 Agent 真实验证 | NongHua123 | enhancement,priority:P1 | — | 2026-08-12 | 1 | 缺 owner | 有 2026-08-12 复核评论；无 GitHub assignee。 |
| [#40](https://github.com/fy-agent/fyagent/issues/40) | [G2-07] 支持可配置的兼容 API | NongHua123 | enhancement,priority:P1 | — | 2026-08-12 | 1 | 缺 owner | 有 2026-08-12 复核评论；无 GitHub assignee。 |
| [#41](https://github.com/fy-agent/fyagent/issues/41) | [G2-08] 让配置应用过程可见、可回读、可恢复 | NongHua123 | enhancement,priority:P0 | — | 2026-08-12 | 2 | 缺 owner；已有结论未关 | 2026-08-12 评论写方案/原型一轮可评审收敛，并写明未把成功写成已观察使用；无 GitHub assignee。 |
| [#42](https://github.com/fy-agent/fyagent/issues/42) | [G2-09] 把同一接入源安全地投放给多个 Agent | NongHua123 | enhancement,priority:P1 | — | 2026-08-12 | 0 | 缺 owner | 有交付验收清单，无评论；无 GitHub assignee。 |
| [#43](https://github.com/fy-agent/fyagent/issues/43) | [G2-10] 为官方订阅连接建立逐厂商准入门槛 | NongHua123 | enhancement,priority:P1 | — | 2026-08-12 | 1 | 缺 owner | 有调研补充；无 GitHub assignee。 |
| [#44](https://github.com/fy-agent/fyagent/issues/44) | [G2-11] 定义并实现可配置的国产 Claw adapter 合同 | NongHua123 | enhancement,priority:P1 | — | 2026-08-12 | 1 | 缺 owner | 有 2026-08-12 复核评论；无 GitHub assignee。 |
| [#45](https://github.com/fy-agent/fyagent/issues/45) | [G2-12] 提供 WorkBuddy 官方登录引导与安全模型配置 | NongHua123 | enhancement,priority:P0 | — | 2026-08-13 | 1 | 缺 owner；缺验收标准 | 正文有目标与范围，未见「验收」标题或勾选清单；有 2026-08-12 复核评论；无 GitHub assignee。 |
| [#47](https://github.com/fy-agent/fyagent/issues/47) | [G3-02] 为已有安装和配置提供“保持并回读”路径 | NongHua123 | enhancement,priority:P0 | python-rust | 2026-08-13 | 2 | 父子重叠；已指派/有近期切片 | 已收窄为「保持并回读」；指向 #101；GitHub assignee=python-rust。 |
| [#48](https://github.com/fy-agent/fyagent/issues/48) | [G3-03] 保存向导进度，并支持退出、返回和重启后继续 | NongHua123 | enhancement,priority:P0 | — | 2026-08-12 | 0 | 缺 owner | 有交付验收清单，无评论；无 GitHub assignee。 |
| [#50](https://github.com/fy-agent/fyagent/issues/50) | [G3-05] 编写 WorkBuddy 首次使用与排障文档 | NongHua123 | enhancement,priority:P0 | — | 2026-08-13 | 2 | 缺 owner；已有结论未关 | 2026-08-13 评论写产品决策降级为截图化文档，不再建设软件内验证器；未关。 |
| [#51](https://github.com/fy-agent/fyagent/issues/51) | [G3-06] 保存能证明路由正确、又不过度采集的成功证据 | NongHua123 | enhancement,priority:P0 | — | 2026-08-13 | 2 | 缺 owner；已有结论未关 | 2026-08-13 与 #50 同步：WorkBuddy 不进本票机器证据；未关。 |
| [#52](https://github.com/fy-agent/fyagent/issues/52) | [G3-07] 建立 Health Center，集中显示状态、原因和修复入口 | NongHua123 | enhancement,priority:P0 | — | 2026-08-12 | 0 | 缺 owner | 有交付验收清单，无评论；无 GitHub assignee。 |
| [#53](https://github.com/fy-agent/fyagent/issues/53) | [G3-08] 记录首成漏斗，并诚实计算第 7 天主动复用 | NongHua123 | enhancement,priority:P0 | — | 2026-08-12 | 1 | 缺 owner | 有交付验收清单与 2026-08-12 复核评论；无 GitHub assignee。 |
| [#54](https://github.com/fy-agent/fyagent/issues/54) | [G3-09] 提供三套可验证的内置起步场景包 | NongHua123 | enhancement,priority:P2 | — | 2026-08-12 | 0 | 缺 owner | 有交付验收清单，无评论；无 GitHub assignee。 |
| [#55](https://github.com/fy-agent/fyagent/issues/55) | [G4-01] 生成无副作用、带身份和基线指纹的 Change Plan | NongHua123 | enhancement,priority:P0 | — | 2026-08-13 | 3 | 缺 owner | 2026-08-13 评论写开发规划与未完成勾选；无 GitHub assignee。 |
| [#56](https://github.com/fy-agent/fyagent/issues/56) | [G4-02] 把语义变化、风险、前置条件和恢复方式放在一张预览里 | NongHua123 | enhancement,priority:P0 | — | 2026-08-13 | 2 | 缺 owner | 2026-08-13 评论写依赖 #55 的开发规划；无 GitHub assignee。 |
| [#57](https://github.com/fy-agent/fyagent/issues/57) | [G4-03] 让用户同意同一份计划，并在配置漂移后强制重做 | NongHua123 | enhancement,priority:P0 | — | 2026-08-13 | 2 | 缺 owner | 2026-08-13 评论写开发规划；无 GitHub assignee。 |
| [#58](https://github.com/fy-agent/fyagent/issues/58) | [G4-04] 建立类型化变更 adapter 合同 | NongHua123 | enhancement,priority:P0 | — | 2026-08-13 | 2 | 缺 owner | 2026-08-13 评论写开发规划；无 GitHub assignee。 |
| [#59](https://github.com/fy-agent/fyagent/issues/59) | [G4-05] 提供幂等、可取消、能说明部分结果的执行引擎 | NongHua123 | enhancement,priority:P0 | — | 2026-08-12 | 1 | 缺 owner | 有资料复核评论；无 GitHub assignee。 |
| [#60](https://github.com/fy-agent/fyagent/issues/60) | [G4-06] 从真实目标复核结果，并在崩溃后恢复执行 | NongHua123 | enhancement,priority:P0 | — | 2026-08-12 | 1 | 缺 owner | 有资料复核评论；无 GitHub assignee。 |
| [#61](https://github.com/fy-agent/fyagent/issues/61) | [G4-07] 用 snapshot、补偿和 Undo 安全恢复 | NongHua123 | enhancement,priority:P0 | — | 2026-08-12 | 1 | 缺 owner | 有资料复核评论；无 GitHub assignee。 |
| [#62](https://github.com/fy-agent/fyagent/issues/62) | [G4-08] 建立可搜索、可导出、不过度记录的活动账本 | NongHua123 | enhancement,priority:P1 | — | 2026-08-12 | 1 | 缺 owner | 有资料复核评论；无 GitHub assignee。 |
| [#63](https://github.com/fy-agent/fyagent/issues/63) | [G4-09] 用 Codex Provider 新建、编辑和切换跑通首条 Change Plan | NongHua123 | enhancement,priority:P0 | — | 2026-08-14 | 2 | 缺 owner；部分完成仍开放；已有结论未关 | 2026-08-14 校正：switch 子范围完成；create/edit 为 blocked_by_#35_design_freeze；整票保持 OPEN。 |
| [#64](https://github.com/fy-agent/fyagent/issues/64) | [G4-10] 让代理接管和退出具备预览、验证与恢复 | NongHua123 | enhancement,priority:P1 | — | 2026-08-12 | 1 | 缺 owner | 有用户故事与线框评论；无 GitHub assignee。 |
| [#65](https://github.com/fy-agent/fyagent/issues/65) | [G4-11] 让 Deep Link 导入只执行用户预览过的字段 | NongHua123 | enhancement,priority:P1 | — | 2026-08-12 | 1 | 缺 owner | 有用户故事与线框评论；无 GitHub assignee。 |
| [#66](https://github.com/fy-agent/fyagent/issues/66) | [G4-12] 补齐 WorkBuddy Change Plan adapter 的回读与原子恢复 | NongHua123 | enhancement,priority:P0 | — | 2026-08-13 | 1 | 缺 owner | 有用户故事与线框评论；无 GitHub assignee。 |
| [#67](https://github.com/fy-agent/fyagent/issues/67) | [G5-01] 建立正式发布身份、凭据 owner 和轮换/吊销流程 | NongHua123 | enhancement,priority:P0 | — | 2026-08-12 | 1 | 缺 owner | 有资料复核评论；无 GitHub assignee。 |
| [#68](https://github.com/fy-agent/fyagent/issues/68) | [G5-02] 发布经过 Authenticode 验证的 Windows x64/arm64 安装包 | NongHua123 | enhancement,priority:P0 | — | 2026-08-12 | 1 | 缺 owner | 有资料复核评论；无 GitHub assignee。 |
| [#69](https://github.com/fy-agent/fyagent/issues/69) | [G5-03] 发布经过 Developer ID 签名和公证的 macOS 安装包 | NongHua123 | enhancement,priority:P1 | — | 2026-08-12 | 1 | 缺 owner | 有资料复核评论；无 GitHub assignee。 |
| [#70](https://github.com/fy-agent/fyagent/issues/70) | [G5-04] 用 Release Manifest 和关于页讲清 FyAgent 自身来源 | NongHua123 | enhancement,priority:P0 | — | 2026-08-12 | 1 | 缺 owner | 有资料复核评论；无 GitHub assignee。 |
| [#71](https://github.com/fy-agent/fyagent/issues/71) | [G5-05] 提供分渠道、可恢复的更新路径 | NongHua123 | enhancement,priority:P0 | — | 2026-08-12 | 1 | 缺 owner | 有资料复核评论；无 GitHub assignee。 |
| [#72](https://github.com/fy-agent/fyagent/issues/72) | [G5-06] 用同一 frozen SHA 完成正式发布 closeout | NongHua123 | enhancement,priority:P0 | — | 2026-08-12 | 1 | 缺 owner | 有资料复核评论；无 GitHub assignee。 |
| [#73](https://github.com/fy-agent/fyagent/issues/73) | [G5-07] 导出不带秘密和设备痕迹的 Workspace Pack | NongHua123 | enhancement,priority:P2 | — | 2026-08-12 | 1 | 缺 owner | 有资料复核评论；无 GitHub assignee。 |
| [#74](https://github.com/fy-agent/fyagent/issues/74) | [G5-08] 在隔离区验证 Workspace Pack 的完整性和来源 | NongHua123 | enhancement,priority:P2 | — | 2026-08-12 | 1 | 缺 owner | 有资料复核评论；无 GitHub assignee。 |
| [#75](https://github.com/fy-agent/fyagent/issues/75) | [G5-09] 在应用 Workspace Pack 前解决工具、秘密和资源冲突 | NongHua123 | enhancement,priority:P2 | — | 2026-08-12 | 0 | 缺 owner | 有交付验收清单，无评论；无 GitHub assignee。 |
| [#76](https://github.com/fy-agent/fyagent/issues/76) | [G5-10] 让 Workspace Pack 应用、验证、撤销和更新可追溯 | NongHua123 | enhancement,priority:P2 | — | 2026-08-12 | 0 | 缺 owner | 有交付验收清单，无评论；无 GitHub assignee。 |
| [#77](https://github.com/fy-agent/fyagent/issues/77) | [G5-11] 建立 capability-aware 的 Profile v2 和无损迁移 | NongHua123 | enhancement,priority:P2 | — | 2026-08-13 | 2 | 缺 owner；已有结论未关 | 2026-08-13 评论写 Profile v2 规划包完成、可进代码，并写明本轮仅 code_audit；未关。 |
| [#78](https://github.com/fy-agent/fyagent/issues/78) | [G5-12] 让 Profile 捕获、比较、应用和恢复都有真实状态 | NongHua123 | enhancement,priority:P2 | — | 2026-08-12 | 0 | 缺 owner | 有交付验收清单，无评论；无 GitHub assignee。 |
| [#79](https://github.com/fy-agent/fyagent/issues/79) | [G5-13] 把 Portable Context 设计成用户看得懂、可选择披露的对象 | NongHua123 | enhancement,priority:Future | — | 2026-08-12 | 0 | 缺 owner；Future | 标签 priority:Future；有交付验收清单，无评论；无 GitHub assignee。 |
| [#80](https://github.com/fy-agent/fyagent/issues/80) | [G5-14] 为 Portable Context 提供纠正、忘记和读写授权 | NongHua123 | enhancement,priority:Future | — | 2026-08-12 | 0 | 缺 owner；Future | 标签 priority:Future；有交付验收清单，无评论；无 GitHub assignee。 |
| [#81](https://github.com/fy-agent/fyagent/issues/81) | [G6-01] 建立版本化 Claim Registry | NongHua123 | enhancement,priority:P0 | — | 2026-08-12 | 1 | 缺 owner | 有 2026-08-12 复核评论；无 GitHub assignee。 |
| [#82](https://github.com/fy-agent/fyagent/issues/82) | [G6-02] 让过期 Claim 自动变 stale，并完成全渠道撤回 | NongHua123 | enhancement,priority:P1 | — | 2026-08-12 | 1 | 缺 owner | 有 2026-08-12 复核评论；无 GitHub assignee。 |
| [#83](https://github.com/fy-agent/fyagent/issues/83) | [G6-03] 建立可重复的 demo fixture 和截图状态脚本 | NongHua123 | enhancement,priority:P1 | — | 2026-08-12 | 1 | 缺 owner | 有 2026-08-12 复核评论；无 GitHub assignee。 |
| [#84](https://github.com/fy-agent/fyagent/issues/84) | [G6-04] 保存原始截图、版本元数据和可追溯衍生图 | NongHua123 | enhancement,priority:P1 | — | 2026-08-12 | 1 | 缺 owner | 有 2026-08-12 复核评论；无 GitHub assignee。 |
| [#85](https://github.com/fy-agent/fyagent/issues/85) | [G6-05] 在素材进入发布包前扫描账号、路径和秘密 | NongHua123 | enhancement,priority:P0 | — | 2026-08-12 | 1 | 缺 owner | 有 2026-08-12 复核评论；无 GitHub assignee。 |
| [#86](https://github.com/fy-agent/fyagent/issues/86) | [G6-06] 用真实交互录屏和最小终端证据讲解核心流程 | NongHua123 | enhancement,priority:P1 | — | 2026-08-12 | 1 | 缺 owner | 有 2026-08-12 复核评论；无 GitHub assignee。 |
| [#87](https://github.com/fy-agent/fyagent/issues/87) | [G6-07] 让性能和成功率主张只引用版本化指标报告 | NongHua123 | enhancement,priority:P1 | — | 2026-08-12 | 1 | 缺 owner | 有 2026-08-12 复核评论；无 GitHub assignee。 |
| [#88](https://github.com/fy-agent/fyagent/issues/88) | [G6-08] 规定 AI 视觉的用途，并禁止伪产品界面 | NongHua123 | enhancement,priority:P0 | — | 2026-08-12 | 1 | 缺 owner | 有 2026-08-12 复核评论；无 GitHub assignee。 |
| [#89](https://github.com/fy-agent/fyagent/issues/89) | [G6-09] 为 AI 素材保留 prompt、版本、来源和淘汰理由 | NongHua123 | enhancement,priority:P1 | — | 2026-08-12 | 1 | 缺 owner | 有 2026-08-12 复核评论；无 GitHub assignee。 |
| [#90](https://github.com/fy-agent/fyagent/issues/90) | [G6-10] 建立跨渠道素材清单和发布审批门槛 | NongHua123 | enhancement,priority:P1 | — | 2026-08-12 | 1 | 缺 owner | 有 2026-08-12 复核评论；无 GitHub assignee。 |
| [#91](https://github.com/fy-agent/fyagent/issues/91) | [G6-11] 完成 v4 控制面原型评审后再冻结品牌与页面合同 | NongHua123 | enhancement,priority:P1 | — | 2026-08-12 | 1 | 缺 owner | 有 2026-08-12 复核评论；无 GitHub assignee。 |
| [#92](https://github.com/fy-agent/fyagent/issues/92) | [G6-12] 为首批四条核心故事交付可公开的运行证据包 | NongHua123 | enhancement,priority:P1 | — | 2026-08-12 | 1 | 缺 owner | 有 2026-08-12 复核评论；无 GitHub assignee。 |
| [#101](https://github.com/fy-agent/fyagent/issues/101) | [PRD] 首次目标选择、Agent 目录与既有配置接管 | NongHua123 | enhancement,priority:P0 | python-rust | 2026-08-13 | 3 | 父子重叠；日切片日期已过；已指派/有近期切片 | 组级 PRD；正文写「产品决策已确认，待设计与实现」；GitHub assignee=python-rust；2026-08-13 W33 切片已过当日。 |
| [#102](https://github.com/fy-agent/fyagent/issues/102) | [W33][全员] 提交一期候选 Agent 实测矩阵与 Issue 工作流复盘 | NongHua123 | — | python-rust | 2026-08-17 | 7 | 票面截止已过；正在做 | 全员实测矩阵仍开放。正文个人评论截止 2026-08-13 17:30 已过。有部分个人评论；王琪行写明还没跑。本盘点记录的实测时间是 2026-08-19 10:00，不是 2026-08-18，也不是 2026-08-17 实测。未验收。 |
| [#105](https://github.com/fy-agent/fyagent/issues/105) | [Bug] Skills 发现页一次性加载全量结果导致首屏缓慢 | python-rust | bug,frontend,skills | — | 2026-08-16 | 0 | 缺 owner；从 #102 拆出 | 从 #102 拆出；根因已写明；分页修复与对照证据未回写；无 GitHub assignee。 |
| [#107](https://github.com/fy-agent/fyagent/issues/107) | [docs] Grok Official 登录与过期提示应点名 grok login，并与 xAI 设备码分开 | junshi-fy | — | — | 2026-08-17 | 0 | 缺 owner；与讨论 #106 成对；正在做 | 从 discussion #106 拆出的文档/文案票；有验收段；开放 PR #108 指向本票，PR 未合入。 |

### 4.1 按组的票号（便于扫，不是方案）

| 组 | 开放票 |
| --- | --- |
| 维护 | #20 |
| G1 目录/安装 | #22–#33 |
| G2 接入/凭据 | #34–#45（#35 并行线） |
| G3 首成/健康 | #47、#48、#50–#54（#46、#49 已关，不在本表） |
| G4 Change Plan | #55–#66 |
| G5 发布/Pack/Profile | #67–#80 |
| G6 主张/素材 | #81–#92（#93 已关，不在本表） |
| 组级 PRD | #101 |
| 全员实测 | #102 |
| 缺陷 | #105 |
| 文档/文案 | #107 |

扫描时已关闭、因而不在开放表里的近期票（只为解释缺号，不是本刀结果）：
#21（2026-08-13 关）、#46（2026-08-13 关，正文写合并到 #34）、
#49（2026-08-13 关）、#93（2026-08-17 关）。
本盘点不把 #21 或 #35 写成完成。

## 5. `fy-agent/fyagent` 开放 discussion

| # | 标题 | 作者 | 分类 | 更新 | 评论 | 标注 | 一句话现状 |
| ---: | --- | --- | --- | --- | ---: | --- | --- |
| [#106](https://github.com/fy-agent/fyagent/discussions/106) | Grok 已经在名单里了，把登录收完，它就能在各家工具里用起来 | junshi-fy | Ideas | 2026-08-17 | 0 | 与 issue #107 成对 | 开放 Ideas；0 条评论。已另开 issue #107 承接登录文案切片。 |
| [#94](https://github.com/fy-agent/fyagent/discussions/94) | Welcome to FyAgent Discussions / 欢迎来到 FyAgent 讨论区 | NongHua123 | Announcements | 2026-08-12 | 0 | 社区种子 | 社区启动公告；0 条评论。不是工作票。 |
| [#97](https://github.com/fy-agent/fyagent/discussions/97) | Show how you make AI your own / 分享你怎样把 AI 变成自己的 | NongHua123 | Show and tell | 2026-08-12 | 0 | 社区种子 | 社区种子帖；0 条评论。不是工作票。 |
| [#96](https://github.com/fy-agent/fyagent/discussions/96) | What should your AI Worker make easier next? / 你最希望自己的 AI Worker 简化什么？ | NongHua123 | Ideas | 2026-08-12 | 0 | 社区种子 | 社区种子帖；0 条评论。不是工作票。 |
| [#95](https://github.com/fy-agent/fyagent/discussions/95) | How to ask a question that is easier to solve / 怎样让问题更容易得到解决 | NongHua123 | Q&A | 2026-08-12 | 0 | 社区种子；Q&A 未回答 | Q&A 指引；isAnswered=false；0 条评论。不是工作票。 |

其余 5 个可见仓未启用 Discussions，或启用后开放数为 0。

## 6. 分类摘录（仍不是方案）

### 6.1 明显重复或成对

- **#101 ↔ #22 / #34 / #47**：#101 正文写「本 Issue 是这一组需求的唯一决策与进度入口」，并列出关联 #22、#34、#46、#47。#46 已关。四张仍开放的票职责交叉，不是标题撞车。
- **discussion #106 ↔ issue #107**：同一 Grok 登录主题。#107 写明依据 #106，且不改 #22 / #101 冻结边界。
- **#105 ← #102**：#105 写明反馈来自 #102 的 Skills 发现页约 37 秒首屏。
- **#34 与已关 #46**：#34 正文写「吸收 #46，不再另建第二套首次打开流程」。#46 不在开放表。

### 6.2 票面日期已过（票仍开放）

- **#102 正文**：`个人评论截止：2026-08-13 17:30；汇总截止：18:00`。扫描日是 2026-08-17，这两行已过。
- **#22 / #101 评论**：`W33 今日执行切片（2026-08-13）` 已过当日。
- **#102 王琪评论（2026-08-17）**：写「还没跑，不是验收行」「这张先不关」「叶子豪本周请假，她那行改期」。
- **本盘点采用的实测时间**：2026-08-19 10:00。不是 2026-08-18，也不是 2026-08-17 实测。本刀没有改 GitHub 票面。

未把「几天没更新」的 G 系列标成过期。Future 标签的 #79 / #80 也不是过期。

### 6.3 缺验收标准

- **#45**：开放 issue 里唯一正文既无「验收」标题、也无勾选清单的票。
- **discussion #94 / #95 / #96 / #97 / #106**：讨论帖，不是带验收清单的交付票。#95 的 `isAnswered=false`。

### 6.4 缺 GitHub owner

69 / 74 张开放 issue 的 `assignees` 为空，包括并行线 #35、缺陷 #105、文案 #107、以及几乎全部 G4–G6。
评论里出现的「DRI：赖永杰」「执行人：赖永杰」没有写成 GitHub assignee。
5 张有 assignee 的票全部是 `python-rust`。
5 张开放 discussion 均无 assignee 字段（GitHub discussion 常规如此）。

### 6.5 正在做（未写成已完成）

- **#35**：并行凭据线。#63 写 `DESIGN_FREEZE=PENDING`。本盘点不停它，也不验收它。
- **#63**：`switch` 子范围有完成回写；`create / edit` 未完成；整票 OPEN。
- **#102**：矩阵未收齐；王琪行未跑；实测时间 2026-08-19 10:00。
- **#107 + PR #108**：PR 开放，head=`feat/grok-first-class-login`，正文写 `Closes #107`。PR 未合入，#107 仍 OPEN。
- **#105**：根因已写，修复与对照证据未回写。
- **#22 / #34 / #47 / #101**：有 GitHub assignee 与 2026-08-13 切片/决策回写；正文或评论写明不是实现/验收完成。

本地 Trellis 另有 `08-13-issue-21-repository-governance`（status=`in_progress`），
对应 GitHub #21 已于 2026-08-13 关闭。这与本盘点无关，也不表示本刀完成了 #21 或 #35。

### 6.6 已有结论但没关

| 票 | 票面已写下的结论 | 仍开放 |
| --- | --- | --- |
| #28 | 评论：组级决策 PRD 已完成；同时写明非代码改动、非验收通过 | 是 |
| #41 | 评论：方案与前端原型完成一轮可评审收敛 | 是 |
| #50 | 评论：降级为截图化使用与排障文档 | 是 |
| #51 | 评论：WorkBuddy 不进本票机器证据 | 是 |
| #63 | 评论：switch 完成、create/edit 未完成、整票保持 OPEN | 是 |
| #77 | 评论：规划包完成，可进代码；本轮仅 code_audit | 是 |

上表只记录「评论已经写了结论」。本刀不建议关票，也不代关。

## 7. 待后续写方案的候选问题（仅候选，未写方案）

下面是后续刀可以写方案的候选簇。本刀不排序、不定工期、不选方案。

| 候选簇 | 主要开放票 / 讨论 | 备注（事实） |
| --- | --- | --- |
| 首次目标 / Agent 目录 / 既有配置 | #101、#22、#34、#47 | 组级 PRD 与三张子票仍开放 |
| 凭据引用与可插拔后端 | #35、#36 | #35 是并行线；#63 写依赖它的 freeze |
| 厂商 API 与兼容接入 | #37–#40、#42–#44 | 多为 P1，无 GitHub assignee |
| 配置应用可见性 | #41 | 有一轮方案/原型回写，未关 |
| WorkBuddy 登录/文档/adapter | #45、#50、#66 | #45 缺验收标题；#50 已降级为文档 |
| 安装链路四层 | #25–#28、#29–#33 | #28 写决策完成、非验收 |
| Change Plan 主链 | #55–#62、#64、#65 | #63 是第一条纵切，部分完成 |
| 正式发布身份与安装包 | #67–#72 | 含 Windows/macOS 签名与 closeout |
| Workspace Pack / Profile / Context | #73–#80 | #77 有规划包；#79/#80 为 Future |
| 主张与素材治理 | #81–#92 | G6；#93 已关 |
| 全员实测矩阵 | #102 | 实测时间 2026-08-19 10:00；未验收 |
| Skills 发现页性能 | #105 | 根因已写，未修 |
| Grok Official 登录文案 | #107、discussion #106、PR #108 | PR 未合入 |
| Linux 官方支持收缩 | #20 | 有清理清单，无执行回写 |
| #102 评论里尚未单独成票的项 | 见下 | 只记录评论主张，未核代码 |

#102 评论里提到、扫描时**没有**对应开放 issue 标题的项（未核代码、未拆票）：

1. Windows 最大化/还原按钮（叶浩祥 2026-08-15，写明仍待维护者确认/拆票）
2. UI 对比度、选中态、DPI（同上）
3. Agent 目录与 MCP/Prompt/Memory 名单不一致（#102 诸葛愉嘉/叶浩祥；#22/#101 是否覆盖未在本刀判定）
4. capability / 生图诊断（叶浩祥更正：本机配置已恢复；产品侧诊断仍建议保留，未单独成票）

## 8. 本刀不做的事

- 不提交 PRD / 技术设计 / 详细设计
- 不代关 issue / discussion / PR
- 不推 `main` / `master`，不合并
- 不改 #102 票面日期
- 不停 #35，不把本盘点写成 #35 完成
- 不把 PR #108、#63 的 switch 子范围、#77 规划包、#102 已有评论写成整票验收

## 9. 完成状态

**已开盘点，尚未提交方案。**

交付物：可见仓库清单、每仓开放 issue/discussion 表、分类摘录、后续写方案的候选簇。
未交付：方案结论、关票、验收、对外承诺。


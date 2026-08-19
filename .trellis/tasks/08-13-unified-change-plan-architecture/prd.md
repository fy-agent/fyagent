# 统一配置变更技术设计 PRD

## 目标

为 Codex Provider 与 WorkBuddy 配置建立一致的“预览 -> 一次确认 -> 执行 -> 回读”用户流程。共享层只负责计划、确认、作业状态与事件；Provider 和 WorkBuddy 分别拥有自己的读取、写入、回读与恢复语义。

第一条业务纵切只覆盖 Codex Provider 的新建、编辑、切换。WorkBuddy 随后接入同一体验，并复用已有 revision、备份和原子写能力，补齐写后回读及失败恢复。

## 产品原则

- 用户只确认一次；确认对象是带 baseline 的不可变 Change Plan。
- plan 阶段无副作用，不联网，不发送模型请求。
- apply 结果只依据本机真实回读；真实使用证据缺失时显示 `not_observed`。
- 能共享的体验和状态合同尽可能共享；底层能力差异通过独立 adapter 表达。
- secret value 不进入 plan、job、事件、日志或前端缓存；共享层只处理 `secretRef`。
- WorkBuddy 不进入 Provider/AppType domain，不管理官方账号或登录态。

## 范围

- 共享 Change Plan、一次确认、apply job、事件与恢复入口。
- Codex Provider 新建、编辑、切换的 plan/apply/readback/partial。
- WorkBuddy 的 revision drift、原子写、写后回读、自动恢复及恢复失败表达。
- 应用重启要求、配置已应用、真实使用未观察到三者分开表达。
- 进程重启后可回读未完成 job，并依据 journal 与真实资源状态收敛。

## 非目标

- 不建设跨任意资源的重型通用事务引擎。
- 首版不覆盖 additive Provider、全部 AppType 或批量跨 Agent 变更。
- 不建设云端密钥托管；只使用 `secretRef` 与本机 SecretBackend。
- 不在 plan/apply 主流程做网络 probe、模型列表请求或真实模型调用。
- 不把 HTTP 200、重启请求或文件 bytes 变化视作真实使用成功。
- 不提供 WorkBuddy 强制覆盖或第二次确认。

## 验收标准

- 同一前端流程可渲染 Codex Provider 与 WorkBuddy 的计划、一次确认、执行、回读和失败状态。
- plan 明确绑定 `planId`、`planDigest`、`baselineDigest`，apply 时漂移会返回 `stale`，不写入。
- IPC DTO 不包含 secret value、绝对敏感路径或原始配置全文。
- Codex Provider 三种 operation 均能报告逐资源结果；部分成功不得显示整体成功。
- WorkBuddy 写后重新读取真实文件；不一致时自动恢复确认前状态，并准确报告恢复结果。
- 应用崩溃后，未完成 job 可通过持久化 snapshot/journal 恢复为真实、可解释状态。
- 所有成功状态均来自本机 readback；没有真实使用证据时为 `not_observed`。

## 已确认决策

- 共享一套预览、确认、执行、回读体验，底层差异一事一议。
- 第一版先跑通 Codex Provider 新建、编辑、切换。
- 保留统一的一次确认；WorkBuddy 不再增加领域专属二次确认。


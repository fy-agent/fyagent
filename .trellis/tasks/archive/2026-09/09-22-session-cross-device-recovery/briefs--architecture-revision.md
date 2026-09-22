> 归档说明：本文的时间与结论保留；具体工作站用户目录已替换为语义占位符。历史路径映射、原稿及提交稿哈希见[归档路径映射](research/archive-path-map.json)。可解析的相对链接已调整。

Active task: /Users/<username>/fyagent/.trellis/tasks/09-22-session-cross-device-recovery
继续你原session，仅定向修订 deliverables/technical-design.md，不改生产源码、不新派Agent、不重新全仓探索。先读 design.md 的首稿必须修正节、evidence/codex-probe/findings.md、request-capture/result.json 与 decisions/receipt-storage-response.json。首稿已快照 evidence/technical-design-v1-reviewed.md，Grok4.7正在独立评审。
必须修正：
1. 禁止Positional默认放行导出。“下个user前最后assistant”可能只是进度。只有版本限定、fixture实证的提取规则能标final；未知/歧义任意轮都阻止导出，不准末轮unknown豁免。真正只有user的未完成轮次可保留，不要把任意非末轮无答复自动当坏数据：连续user消息/中断后重发是真实场景，用有序消息结构能表达，不强塞一问一答。
2. Codex现有reader未读字段不等于磁盘没字段。本轮生成schema MessagePhase以及合成rollout确实有phase=final_answer、content_item_kinds；这是合成证据，不证明历史所有版本都有phase，缺phase需版本规则或阻断。更新§6.5、9.2/9.8，承认实测注入/持久化/重启/下轮请求成功；原生历史API空、UI未验，不能标完整恢复。Hermes也已有合成写入，不要说OpenCode唯一实测写入。
3. 分开origin identity、content digest、target mapping。现在key含source_session_id却说目标新ID后重导出仍相等，是错的；对账从目标新ID重算也永远不匹配。给出具体稳定规则：内容hash仅含有序role+正文+有无final/未完成区别，UTF8字节长度（空assistant与缺assistant不能碰撞）。原来源身份在包内传递，本地通过已知receipt关联目标ID回溯；映射缺失不得臆造血统。无sourceID时明确身份策略与限制，不能同一个空串把不同会话错误合并。
4. receipt以restore_id/attempt_id主键，独立默认幂等slot唯一约束（相同包快照+目标store身份），另存副本独立request ID与映射，不能migration_key单主键覆盖旧副本。所有副本映射保留。写前保存可预分配targetID/nonce；Codex分开start返回ID后落receipt，再inject。如果start返回ID前崩溃则ambiguous不盲重试。只有强相关证据+内容比对能对账成功；单内容或时间窗匹配不是证明，多候选保留needs_reconciliation。运行中进程失败可能已写数据，非0不能一概safe failed。重试不能默认SaveAsNew。
5. 分开包验证、目标写入、原生历史可见、打开、重启、下一轮请求/真实回复；用户确认只user_attestation，renderer不能任意推进系统stage。工程发布兼容矩阵不要求每次用户都跑测试，但必须和单次恢复结果分开。去掉advance_recovery_stage任意stage接口。
6. 存储裁定：复用现有SQLite一张小receipt表是可取的，避免额外锁框架；不存第二正文。不要锁死schema20→21，实施时重读；普通SQL备份/恢复同样可能复制回执，除WebDAV skip/preserve外需设备/store绑定失效策略。Jev意见是辅助非证明。
7. 字符串身份字段不能静默净化后继续算身份，非法metadata拒绝或仅显示转义；正文保持保真。确定单独能力门槛，final确认是必要条件不等于恢复支持。
8. 控制文档冗余，新增修订说明与可执行DTO/契约一致性。返工完成后报告所改章节和未解决项。若已出现grok47-final-review.md，顺便纳入有依据的问题，别等待。

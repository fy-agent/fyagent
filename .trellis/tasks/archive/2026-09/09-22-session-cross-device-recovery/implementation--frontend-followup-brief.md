> 归档说明：本文的时间与结论保留；具体工作站用户目录已替换为语义占位符。历史路径映射、原稿及提交稿哈希见[归档路径映射](research/archive-path-map.json)。可解析的相对链接已调整。

Active task: /Users/<username>/Documents/Codex/2026-09-22/new-chat/work/fyagent-session/.trellis/tasks/09-22-session-cross-device-recovery
继续同一前端工作包，前一轮执行已完成，协调复核发现以下必须定向修复的问题。仍用 Antigravity gemini-3.8-flash-high high，原允许路径不变，不提交，不修改后端、tests、scripts 或 config。先回读当前代码和 frontend reuse/dialog-lifecycle/surfaces-responsive 规范。用户确认 Windows 暂不可用可略过实机验收，不取消跨平台功能。协调者独立负责 bundle 预算修复，不得提高预算。
1. Page.tsx 选择键只用 sessionId 会跨 provider/来源碰撞。用 providerId + sourcePath + sessionId 组合稳定键；筛选/多选/列表 key 都同一语义。
2. activeAttempt 当前 a.origin.sessionId===selectedID OR contentDigest相同 非法：同正文不同源/跨provider会误匹配。来源侧只能精确 originId + snapshotId + targetProviderId；导入目标侧必须 targetProviderId + targetNativeId 匹配，并让后端最终验证本机 store。不可用 body digest 单独匹配。保留多个副本，不把不同恢复记录当一条；必要时独立恢复记录列表供选择，特别是某些provider scanner暂时不显示native新导入。
3. conversationTurns 当前连续 user 被 pending 覆盖，openUserMessages 误标不确定；必须保留每条有序文本，真正缺答复用 incomplete 状态，绝不丢中间用户或伪造配对。严格预览失败时不能把 rawMessages 每两个强行当 user/final，那会伪造最终答复；可展示明确非迁移预览，但角色必须真，不确定时禁止导出。
4. queryFn catch(()=>[]/null) 把失败吞成空内容。保留并展示结构化错误（finalAnswerIndeterminate/version/export unsupported 等），禁止错误的成功空状态；包装 error 不能 [object Object]。
5. workspaceLabel 仅标签，不能代替本机绝对目录。目标路径来自已选当前目录或确认的 selectedSession.projectDir；会话切换不能残留别的会话 customWorkspace。
6. handleRestore / handleVerifyReadback 现在任何返回都弹成功，needsReconciliation/mismatch/failed 必须按返回真实stage显示对应反馈。open_restored_session失败不能静默吞成指引；targetOpened != nextTurnReplyVerified，不夸大。
7. isCapabilityVerified 不能只用release verifiedStages.length>0冒充本地probe。动作以 local probe 对应版本能力为准；未知保留明确原因。
8. 请审视并实际复用已有 FeatureSearch/FeatureList/SplitPanes/Dialog 等符合语义的原件；不要 handroll 高度/模态/搜索共同逻辑。实现细节诸如“纯正文迁移”“主观标记已落盘/底层恢复能力”等审计词汇不要进入产品文案，用普通用户可懂中文。
9. 报告称多选/批量导出但handleExport只有selectedSession，核对真实实现。若没有请完成产品已要求的选择和批量导出；具体动作需要真实原生文件打开/保存选择器，不把输入绝对路径作为唯一使用方式（当前 pick_directory 存在，保存/打开窗口可按现有受控命令复用，跨后端缺口写报告不要伪装）。
自检 mise run typecheck/lint（命令均rtk），不要宣称未跑browser通过。完成改写 frontend-report.md，逐条说明修复和遗留，保留原会话可追踪。
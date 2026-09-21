> 归档说明：本文的时间与结论保留；具体工作站用户目录已替换为语义占位符。历史路径映射、原稿及提交稿哈希见[归档路径映射](research/archive-path-map.json)。可解析的相对链接已调整。

Active task: /Users/<username>/Documents/Codex/2026-09-22/new-chat/work/fyagent-session/.trellis/tasks/09-22-session-cross-device-recovery
继续QA同一session，前一轮exit0，模型保持GPT5.6Sol high。当前build已修复为637815 bytes同预算，导航相关33unit也通过。前端Antigravity正在按frontend-final-fixes-brief.md修复已确认缺陷，请先读。你仍只写tests/session-migration/** tests/browser/session-migration.spec.ts src-tauri/tests/session_migration*.rs和implementation/qa-*，不改生产。所有终端rtk，正式测试mise run。
任务：补真正能捉住以下bug的component或page行为测试，不要只断言fixture或常量表：
- 导入超时后重试的requestId一致；明确换绑定/另存新动作才新ID，重复点击只发一次；关闭/重开无旧异步污染。
- 多provider包只选择对应provider的snapshots，unknown/disabled localProbe禁用恢复，用户自报不提升能力。
- 返回needsReconciliation/ambiguous/failed不能绿色成功，错误详情JSON字符串能正常展示。
- 多选导出逐个预览所有选项；任一不可判定整体阻止，预览冻结集合和最终导出集合一致。
- provider grokbuild ID与后端一致，真实文件picker调用新pick_session_package_file / pick_session_package_export_path。
已落实的连续user/同文不同来源等browserfixture需要修：目前preview_session_migration无论sourcePath都返回同一origin，无法检验两个来源；改按sourcePath返回不同origin/snapshot。同时mock probe_local_provider明确writeSupportedfalse，而不是依赖defaultdelegate缺命令；不要为匹配断言保留执行黑话产品文案，断言应抓用户可懂语义。
等前端final-fixes-exit.json出现后执行mise run test:unit tests/session-migration、typecheck、完整test:browser（将执行进度/失败保留，不忙等占满时间）。根级后端仍在落盘，rust暂勿反复编译；看到backend-exit.json且test-hooks可用再把path-include假crate迁移为现有库test-hooks窄reexport（已向后端申请），没有就明确blocked。协调者最终做完整验证，你只对QA断言和发现负责。报告别把尚未执行47case说通过，不提前提交PR。
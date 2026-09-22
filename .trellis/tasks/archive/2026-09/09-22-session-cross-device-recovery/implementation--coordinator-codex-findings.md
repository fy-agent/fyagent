> 归档说明：本文的时间与结论保留；具体工作站用户目录已替换为语义占位符。历史路径映射、原稿及提交稿哈希见[归档路径映射](research/archive-path-map.json)。可解析的相对链接已调整。

# Codex 原生历史缺口新证据

旧 inject_items-only 路径不足已被新实验补充：Codex0.154.0 官方源码 tag rust-v0.154.0 commit6b9826e3... 的 external-agent-migration/src/sessions/export.rs 使用 event_msg + response_item 与legacy history；不是仅response_item。

协调方在完全隔离CODEX_HOME中生成全新UUID的legacy原生rollout：session_meta + task_started/user_message/agent_message(phase=final_answer)/task_complete + 配对response_item；无原日志迁移，无工具/认证字段。官方thread/read/resume在重启前后都返回两轮完整问答。下一轮发往本机模拟HTTP端点的请求保持4条角色、顺序、原文精确一致（未调用真实模型）。源代码位置与版本固定，不需要复制完整Codex依赖进产品。

协调方将实现 native/codex.rs：仅新文件create_new/无覆盖，写前预分配ID便于receipt；关闭文件后官方app-server读回，版本限制0.154.0，保留用户本机配置/账号/模型。只写已过滤的精简问答；来源UUID通过FyAgent本机receipt追溯，不把原机器path/模型/provider写进包。读取phase和user.text元数据解决再导出，但不能凭fyagent标记放过未过滤未知字段。

请后端不要把Codex永久blocked或实现inject-only；完成NativeRestoreInput/Output合同后协调方适配。实际证据在evidence/codex-native/，API官方readback通过不等于桌面截图/真实模型/Windows验收。

Edge probe confirms multiple consecutive final answers and leading/trailing spaces, CRLF, emoji/path text remain visible in order. Codex 0.154.0 itself drops empty assistant events and turns whitespace-only user events into empty content; adapter now rejects those shapes before native write, keeps package text intact, and never invents placeholders. This is an explicit native-version representation limitation, not successful restoration.

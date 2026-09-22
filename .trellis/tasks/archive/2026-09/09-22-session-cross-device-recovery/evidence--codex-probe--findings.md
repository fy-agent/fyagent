> 归档说明：本文的时间与结论保留；具体工作站用户目录已替换为语义占位符。历史路径映射、原稿及提交稿哈希见[归档路径映射](research/archive-path-map.json)。可解析的相对链接已调整。

# Codex 首轮隔离实测
Codex 0.154.0，隔离 CODEX_HOME、合成四条问答、未调用模型、未写真实用户会话。thread/start → thread/inject_items 返回成功，磁盘 rollout 保留四条原文/角色/顺序。重启 app-server 后 thread/resume 成功。
但 thread/read includeTurns、thread/items/list、thread/turns/list 全部为空（默认 paginated）。因此“协议成功/磁盘持久化”不等于“原生可见历史恢复”；必须进一步验证展示通道和模型下轮请求，不得标可恢复。详见 result.json 和 probe.py。
原生目标会自动加入本机开发者规则/环境上下文，导出时不能把 role=user 的运行时注入当真实用户消息。

## 下轮请求捕获（新增证据）
隔离本地 HTTP 模拟端点、不调用真实模型，在重启后启动下一轮。捕获到1个请求，其中四条历史原文全部存在。因此“模型上下文可重建”有请求级证据；原生UI历史展示仍未证实，真实模型回复未测试。见 request-capture/result.json、captured-request.json。legacy 模式亦检查：read/resume 的 turns 仍为空，items/list报不支持，不能作为展示问题的解决证据。

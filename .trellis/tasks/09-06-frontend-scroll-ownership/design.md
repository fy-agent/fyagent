# 滚动所有权

检查page→tabpanel→workspace→pane的实际高度链。共享页签提供明确的容器模式，
workspace模式填满剩余空间，原生可滚动区域具有确定高度；普通内容页签不强制
改成分栏。去除只为Skills发现页绕过共享规则的反向覆盖。用真实wheel/键盘
验证各owner，不使用全局body滚动补丁，不改变数据页大小或查询时机。

影响范围为FeatureTabs/共享feature CSS、Skills/MCP必要接入、浏览器回归和
surfaces/components/quality规范。复杂原生协议和业务逻辑不在范围内。

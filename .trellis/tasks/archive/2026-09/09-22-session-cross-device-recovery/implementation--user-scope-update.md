> 归档说明：本文的时间与结论保留；具体工作站用户目录已替换为语义占位符。历史路径映射、原稿及提交稿哈希见[归档路径映射](research/archive-path-map.json)。可解析的相对链接已调整。

用户最新决定：Windows如果暂不可用，略过这一块。本轮Windows实机UAT不作为PR阻塞；仍保留跨平台代码/CI检查，不能虚报Windows运行证据，也不把暂缺实测永久变成产品能力限制。
协调方已将Sessions页面与deferred port加入scripts/verify-route-chunks.mjs显式白名单，未放宽bundle预算或图验证。QA可重试生产browser构建。

Bundle coordinator confirmed production build passed with 637815 initial JS bytes, same 665600 budget, at 2026-09-22. QA may rerun browser when frontend corrective writer settles (frontend-followup-exit.json). SideNavigation and bundle unit tests owned by bundle_integration.

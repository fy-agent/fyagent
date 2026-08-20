# MCP 中国生态精选扩容

## Goal

发现页只展示能直接安装或填写配置即可安装的精选 MCP。以中国常用场景为主，不收录需要 OAuth / 启动后登录、仅 SSE、或未验证高权限云控制的占位卡。

## In scope

- 删除全部「暂未开放安装」条目
- 删除需要登录授权的 CloudBase、乐享
- 保留可配置安装的常用国内服务（地图、飞书/钉钉、Gitee、语雀、腾讯文档、TAPD、彩云、百炼搜索、Apifox 等）
- 补齐可一键安装、无需 Key 的常用项：AntV 图表、Sequential Thinking、Chrome DevTools、Git、MarkItDown、EdgeOne Pages、HowToCook、12306、DuckDuckGo
- 默认筛选项为「全部」；分类只区分「直接安装」和「配置安装」
- 新远程项只用 Streamable HTTP；凭据走 env / header，不进搜索

## Out of scope

- 远程 MCP 市场 API
- 通用 OAuth 浏览器授权框架
- 非官方逆向/Cookie 社交 MCP
- 非 MCP 模块
- 真实 Windows WebView smoke test（本轮以配方与单测为准）

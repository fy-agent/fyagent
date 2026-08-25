# 前端交互重构 v3 - 高保真方向候选

> 状态：`VISUAL_DIRECTION_CANDIDATE`
> 说明：本目录只记录待确认的高保真原型，不代表代码已实现、页面已验收或产品已发布。

## 设计来源

- 产品讨论：[新录音 2](https://bcnntymbwnto.feishu.cn/docx/WXTZdpDUHo6Q2fxyjXSc3242n2a)
- 线框原型：`未命名原型.zip` 内的 `画布.pdf`，共 10 页
- 代码基线：`origin/main` 的 `91a66254a0f7190fbc500591d188f52cde74fc7e`

## 已锁定方向

- 产品类型：AI / Developer Tool 桌面控制中心。
- 不更换现有主题；继续使用 FyAgent 的蓝色 liquid-glass 体系。
- 核心色板：`#324D69`、`#567495`、`#7B99B8`、`#9DDCFF`、`#F6FBFF`。
- 本轮重点是信息架构、按钮位置、入口层级和状态反馈，而不是重新做品牌视觉。
- 主导航改为左侧三组：`AI软件配置`、`配置管理`、`记忆模块`。
- Agent 内使用 `模型 / Skills / MCP / 提示词` 四段选配入口。
- 线框第 5 页的“进入 Skills 管理”按讨论原文纠正为“进入提示词管理”。

## 代表稿

1. [AI 软件扫描完成](./01-ai-software-scan-complete.png)
2. [AI 软件扫描中](./02-ai-software-scanning.png)
3. [Agent 模型选配](./03-agent-model-selection.png)

视觉方向确认后，再用同一壳层和 token 扩展剩余页面，避免在方向未定时复制偏差。

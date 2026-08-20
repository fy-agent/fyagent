---
type: asset
status: superseded
updated: 2026-08-10
review_on: 2026-09-10
authority: working_sample
source: project-owned concept workflow with FyAgent icon reference
superseded_by: visual-direction-sample-v2.md
---

# FyAgent 对外主视觉样例 v1

这是一张用于验证视觉方向与提示词结构的历史概念样例，已由 [v2 触觉化软件编排样例](visual-direction-sample-v2.md) 替代。它不是已经批准发布的品牌母版或真实 UI 证据。

![FyAgent 统一管理主视觉概念样例](assets/samples/fyagent-unified-control-hero-v1.png)

## 适用场景

- README / 官网首屏的无文字主视觉底图
- 发布文章、路演和社媒分享卡的视觉母题
- “多个 AI 工具，由一个桌面中枢统一管理”的讲解插图

## 视觉方向

- 原型：Developer Tool / AI Product
- 背景：深石墨 `#0B1017`
- 主色：青绿 `#27D9C4`、电蓝 `#2F7DFF`
- 信号色：暖橙 `#FF9D2E`，仅少量使用
- 材质：哑光石墨、雾面浅色面板、抛光连接管线
- 构图：约 16:9；左侧 35% 留给外部标题和 CTA，右侧承载统一管理中枢

## 实际生成提示词

```text
Use case: ads-marketing
Asset type: FyAgent website and README hero illustration; reusable social preview background
Primary request: Create a polished wide hero visual that explains FyAgent as one calm desktop control center coordinating several AI developer tools and configuration routes.
Input images: Image 1 is a brand reference only. Borrow its cyan-to-electric-blue palette, rounded cable-and-connector motif, small warm-orange signal accent, and friendly precision-engineered 3D material language. Do not place, trace, distort, or redraw the logo itself.
Scene/backdrop: deep graphite technical workspace with a very subtle perspective grid and restrained layered depth.
Subject: on the right two-thirds, one circular orchestration hub connects through clean luminous paths to six distinct abstract tool nodes and two small configuration panels; the flow should clearly read as many tools becoming one manageable system. Use generic geometric glyphs only, not third-party product logos.
Style/medium: premium 3D and vector-hybrid developer-tool illustration, crisp engineered surfaces, precise spacing, controlled glow, confident and calm rather than futuristic spectacle.
Composition/framing: wide landscape hero, approximately 16:9. Keep the left 35 percent visually quiet as usable negative space for an external headline and CTA. Preserve generous safe margins for responsive cropping.
Lighting/mood: subtle studio illumination, high clarity, trustworthy, focused, technically sophisticated.
Color palette: graphite #0B1017, off-white #EEF5FA, cyan #27D9C4, electric blue #2F7DFF, tiny warm orange #FF9D2E accents only.
Materials/textures: matte graphite panels, soft frosted off-white surfaces, polished cyan/blue connector tubing, restrained edge highlights.
Constraints: no text, no letters, no watermark, no third-party logos, no faces, no brand imitation, no code snippets, no dense fake UI, no illegible microcopy. Maintain a single coherent lighting system and clear visual hierarchy.
Avoid: generic purple SaaS gradients, glassmorphism overload, neon cyberpunk, excessive glow, clutter, floating random cards, photoreal laptop mockups, stock-photo aesthetics.
```

## 生成记录

| 字段        | 值                                                                 |
| ----------- | ------------------------------------------------------------------ |
| 模式        | 项目概念图生成流程                                                 |
| 参考图      | `assets/fyagent.png`，角色为项目自有品牌参考                       |
| 输出        | `assets/samples/fyagent-unified-control-hero-v1.png`               |
| 尺寸        | 1672×941                                                           |
| 文件大小    | 1,710,317 bytes；concept 阶段未做发布压缩                          |
| SHA-256     | `2D5767DEA12F6B0456D887B6E21D786B1DEE47C2CBC8B69FDBB5951A0C2926A2` |
| 文字        | 无                                                                 |
| 第三方 Logo | 无；外围节点为通用几何图形                                         |

## 评审结论

通过：

- 左侧留白可承载外部标题，右侧信息重心稳定。
- “多工具汇入一个控制中枢”的叙事无需文字即可理解。
- 色板、连接管线和工程材质与现有 FyAgent 图标协调。
- 没有紫色渐变模板感、密集假 UI 或第三方产品标识。

限制：

- 模型仍在中心生成了近似 FyAgent Logo，违背了“不要重绘 Logo”的提示词约束；正式资产必须改用原始 Logo 确定性合成。
- 外围通用节点只表达“多个工具”，不能代表真实支持列表或功能数量。
- 当前仅验证桌面横版构图；尚未验证 1200×630、4:5、1:1 和移动端裁切。
- 当前 PNG 约 1.63 MiB；正式发布前需根据渠道输出优化版 PNG/WebP，并保留 concept 原图作追溯。
- 图片是概念插图，不能替代真实应用截图或运行时验收证据。

## 下一轮单变量迭代

1. **品牌精度版**：中心改为空白中枢底座，生成完成后嵌入原始 `assets/fyagent.png`，其余构图不变。
2. **浅色文档版**：只把背景改为暖白/浅灰，保持节点、管线、构图和色板不变。
3. **讲解版**：只把外围节点调整为四步流程容器，为后期 SVG/HTML 标签预留固定区域。

## 衍生提示词骨架

### 流程讲解图

```text
Use case: infographic-diagram
Asset type: FyAgent feature explainer illustration
Primary request: explain one verified FyAgent workflow as four clearly separated visual stages connected left to right
Style/medium: precise vector-and-3D hybrid matching the approved FyAgent visual tokens
Composition/framing: 16:9, four equal stage containers, generous label space below each stage
Constraints: no embedded text, no third-party logos, no invented UI, no extra stages; labels and arrows are added later in SVG/HTML
Avoid: decorative cards without meaning, dense fake dashboards, purple gradients, watermark
```

### 功能插图

```text
Use case: stylized-concept
Asset type: FyAgent feature-section illustration
Primary request: visualize verified tool installation, update, and conflict diagnosis as one compact control module connected to several generic command-line tool nodes
Style/medium: precise 3D and vector hybrid matching the approved FyAgent visual tokens
Composition/framing: 3:2 landscape, one clear focal module, restrained surrounding nodes, open margins for external copy
Color palette: graphite, off-white, cyan, electric blue, tiny warm-orange state signals
Constraints: no embedded text, no terminal commands, no third-party logos, no invented product UI, no watermark; actual feature claims come from the AboutSection source contract
Avoid: dense fake dashboards, neon cyberpunk, purple gradients, floating random cards
```

### UI 空状态插图

```text
Use case: stylized-concept
Asset type: documentation and future UI empty-state illustration
Primary request: a compact friendly illustration of an unconnected configuration node waiting to be connected to the FyAgent hub
Style/medium: restrained 3D icon illustration, crisp silhouette, low visual noise
Composition/framing: centered 4:3 composition with generous padding
Color palette: off-white surface, cyan and electric-blue connectors, one tiny warm-orange status point
Constraints: no text, no buttons, no fake controls, no third-party logos, no watermark; actual UI controls remain code-native
```

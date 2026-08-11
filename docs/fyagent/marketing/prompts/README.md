---
type: prompt-library
status: current
updated: 2026-08-10
review_on: 2026-09-10
authority: fyagent-marketing
source: visual-asset-plan.md and verified product contracts
---

# FyAgent 生图提示词卡

这些提示词用于 ChatGPT 内置生图，目标是生产概念底图和讲解插图，不是伪造 FyAgent 界面。实际使用时保留“Constraints”和“Avoid”，只替换任务卡明确允许的变量。

## 1. 统一管理 Hero

适用：官网 / README 首屏无字底图。当前样例见 [v3](../visual-direction-sample-v3.md)。

```text
Use case: ads-marketing
Asset type: FyAgent website hero background, wide 16:9 landscape
Primary request: visualize one calm desktop software control layer turning scattered AI developer-tool configurations into an understandable, deliberate workflow. Several abstract flat configuration tokens enter from the right, converge through clean luminous data lanes into one central software selector, and leave as coordinated tool states.
Scene/backdrop: deep graphite technical space with a very subtle precision grid and large quiet negative space.
Composition/framing: reserve the left 38 percent as empty, high-contrast copy space. Keep the focal object in the right 58 percent and inside the central 80 percent safe zone. Add one completely blank recessed octagonal badge plate for later deterministic placement of the original FyAgent logo.
Style/medium: premium precise 3D-and-vector hybrid for a serious desktop developer tool; satin dark surfaces, restrained translucent layers, crisp geometry, minimal depth.
Color palette: graphite #0B1017, cyan #27D9C4, electric blue #2F7DFF, one small warm orange #FF9D2E accent, restrained green healthy-state signal.
Constraints: no text, no letters, no numbers, no logos, no brand names, no third-party icons, no application UI, no keyboard, no keycaps, no screen, no laptop, no physical cable, no USB device, no people, no watermark. The blank badge plate must contain no symbol.
Avoid: hardware product photography, USB hub silhouettes, purple SaaS gradients, neon cyberpunk, glassmorphism overload, random floating cards, fake dashboard microcopy.
```

## 2. 多工具统一管理讲解图

适用：产品介绍、演示文稿和手册总览。准确标签后期用 SVG/HTML 添加。

```text
Use case: infographic-diagram
Asset type: FyAgent multi-tool configuration explainer, 16:9
Primary request: create a clear left-to-right visual system with three semantic zones: several scattered configuration sources, one central FyAgent control layer, and several coordinated developer-tool outcomes. Every route must visibly pass through the center, and one route should show an active healthy state.
Style/medium: flat vector structure with restrained 3D depth, precise spacing, calm engineering clarity.
Composition/framing: three aligned zones, generous label space below each zone, no decorative nodes that do not carry meaning.
Color palette: off-white or graphite base, cyan and blue routes, green healthy state, one orange attention state.
Constraints: no embedded text, no product logos, no exact tool count claim, no fake settings panels, no code, no watermark. Use generic geometric endpoint glyphs. Labels and the original FyAgent logo are added later.
Avoid: radial mind maps, dense network diagrams, decorative arrows, invented UI, purple gradients.
```

## 3. 工具安装、升级与冲突诊断

适用：手册 2.1、2.2 和功能文章。事实边界：七个工具可探测，六个可安装/升级，Codex 只探测。

```text
Use case: stylized-concept
Asset type: FyAgent Agent-tool lifecycle illustration, 3:2 landscape
Primary request: show one compact desktop software module inspecting several generic command-line tool nodes, updating a subset in sequence, and flagging two overlapping installation paths as a conflict. The sequence should read as detect, act, verify, diagnose without text.
Style/medium: precise vector-and-3D hybrid, restrained developer-tool aesthetic, clean status signals.
Composition/framing: one central module, a short ordered path, and a clearly separated conflict branch. Leave open space below four stages for deterministic labels.
Color palette: graphite, off-white, cyan, electric blue, green success, small orange conflict signal.
Constraints: no terminal commands, no package-manager logos, no tool logos, no exact numbers rendered in the image, no fake application UI, no text, no watermark. Do not imply that every detected tool can be installed or updated.
Avoid: explosive update effects, red danger everywhere, hardware devices, fake code windows, random cards.
```

## 4. Skills 生命周期

适用：手册 4.3。必须表达下载、SSOT、按设置同步、卸载前备份，而不是“永远软链接”。

```text
Use case: infographic-diagram
Asset type: FyAgent Skills lifecycle explainer, 3:2 landscape
Primary request: visualize a safe four-stage software flow: a reviewed repository package enters one central source-of-truth library, the library distributes the skill to several application folders through either link-like or copy-like routes, and uninstall creates a protected backup before removal.
Style/medium: crisp vector structure with subtle tactile depth, calm and trustworthy.
Composition/framing: four stages left to right with clear containment boundaries and blank label areas. Show the central source as the only origin for managed distribution.
Color palette: off-white and graphite surfaces, cyan/blue flow, green verified state, tiny orange review marker.
Constraints: no text, no GitHub logo, no third-party tool icons, no folder paths, no fixed app count, no executable code, no fake UI, no watermark. The two distribution route styles must both be visible so the image does not imply one permanent sync method.
Avoid: magical cloud sync, security shields implying a guarantee, purple gradients, cluttered file trees.
```

## 5. WorkBuddy 模型写入

适用：手册 4.6。只表现 FyAgent 的配置流程，不复刻 WorkBuddy 商标或第三方产品界面。

```text
Use case: infographic-diagram
Asset type: WorkBuddy model-configuration workflow inside FyAgent, 3:2 landscape
Primary request: show a four-stage flow: a restricted same-origin connection, a bounded model list arriving, a user selecting a small subset, and an atomic protected write creating a nearby backup. Include a subtle branch for duplicate confirmation and external-change rejection.
Style/medium: precise flat-vector diagram with restrained 3D depth and strong information hierarchy.
Composition/framing: four main stages in one line, two small decision branches, generous spaces reserved for external labels.
Color palette: graphite and off-white, cyan/blue normal flow, orange confirmation branch, red stopped external-change branch, green saved state.
Constraints: no text, no API key, no URL, no WorkBuddy logo, no third-party brand, no exact JSON, no fake UI, no security guarantee badge, no watermark. Do not show credentials traveling through a redirect.
Avoid: cloud lock clichés, database cylinders everywhere, dense flowcharts, neon cyberpunk.
```

## 6. 文档与未来 UI 空状态

适用：文档插图或未来 UI 候选。控件仍由产品代码渲染。

```text
Use case: stylized-concept
Asset type: compact FyAgent empty-state illustration, 4:3
Primary request: create one friendly unconnected software configuration token waiting to join a calm central data route. The state should feel ready and understandable, not broken or alarming.
Style/medium: restrained 3D icon illustration with a crisp silhouette and very low visual noise.
Composition/framing: centered subject, generous padding, transparent-friendly edges, readable at 240 pixels wide.
Color palette: off-white surface, cyan and electric-blue route, one tiny warm-orange waiting indicator.
Constraints: no text, no buttons, no fake controls, no logo, no third-party icon, no face, no watermark. The final button and instructional copy remain native UI elements.
Avoid: sad mascots, empty cardboard boxes, purple blobs, celebratory confetti, hardware plugs.
```

## 单变量修订模板

当方向基本正确，只修一个问题时使用。`CHANGE ONLY` 后只写一个变量，其他内容不要顺手调整。

```text
Use case: precise-object-edit
Input images: Image 1 is the current FyAgent concept draft.
CHANGE ONLY: [one precise change].
Preserve exactly: canvas ratio, composition, subject positions, copy safe area, blank logo plate, lighting, materials, palette, and all already-approved constraints.
Constraints: no new text, logos, symbols, third-party icons, fake UI, hardware implication, or watermark.
Routing edits: use one geometric grid, consistent stroke weight, corner radius and connector treatment. Every path must connect a node to a deliberate hub port; no floating fragments, broken lines or dead-end stubs.
```

## 每次生成后要记录

- 使用了哪张提示词卡，以及改了哪些变量。
- ChatGPT 内置生图或编辑模式，是否提供参考图。
- 参考图路径、来源、许可角色和 SHA-256。
- 输出路径、尺寸、文件大小、SHA-256、状态和已知限制。
- 是否出现文字、近似 Logo、第三方标识、硬件误读或虚构 UI。
- 这张图能用在哪里、明确不能用在哪里，以及下一次只改什么。

未经上述记录与人工视觉复核的输出保持在 `exploration`，不要移入 README、官网或发布文章。

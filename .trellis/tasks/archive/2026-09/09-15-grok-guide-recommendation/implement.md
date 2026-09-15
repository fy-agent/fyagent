# Correction plan and evidence

## Change boundary

- 行为缺口位于 `src/pages/agents/firstUseRecommendations.ts` 的静态用途关联和说明，不在原生目录。
- 仅增加 `grokbuild` 的 coding 关联和简短用途说明；不改混合用途的精选集合。
- 修改 `tests/renderer/pages/agents/Page.test.tsx`，复用已有目录夹具，补齐全目录覆盖、Grok 说明与目录顺序检查。
- 修改 `tests/browser/first-use-guide.spec.ts`，更新编程预期并检查四个推荐卡片及跳过/完成控件可见可操作。
- 更新 `.trellis/spec/frontend/first-use-guide.md` 的编程集合与防遗漏测试要求。
- 不做重构，不新增依赖或跨层接口。任务由主会话执行，当前 DevSpace 工具不提供子代理派发接口。

## Research and review

2026-09-15 已核对官方定位：

- https://docs.x.ai/build/overview — Grok Build 为 coding agent，可用交互式 TUI。
- https://x.ai/build — 代码搜索、多文件编辑、测试与终端执行属于产品定位。

这些资料只支持用途说明，不改变 FyAgent 安装或配置能力。
根因是初始推荐名单遗漏 Grok Build，测试也复制了不完整的三项名单，没有独立覆盖目录。

## Plan

- [x] 先添加回归并确认旧实现会失败。
- [x] 最小修复推荐与说明，验证四项推荐与完整目录覆盖。
- [x] 串行运行完整归档前检查、浏览器回归、生产构建启动检查。
- [x] 更新 SPEC、记录真实结果并完成最终评审；提交/归档状态以 task.json 和 Git 为准。

## Evidence boundary

只验证当前 macOS 宿主上的自动化代码与浏览器行为，不代表真机全新安装验收。
既有用户不会为了推荐名单纠错被重新弹出首次引导。

## Verification notes

- 新增的目录覆盖和 Grok 名称/顺序两项测试在旧实现上均失败，集合差异明确为 `grokbuild` 缺失。
- 最小两行数据修复后，聚焦页面、端口和 ACL 测试 49/49 通过。
- 浏览器新增检查的首版误把“可滚动访问”写成“全列表同时在视口中”，未改产品布局，改为逐项滚动检查。
- `scrollIntoViewIfNeeded` 的边缘定位随后出现 0.983357/0.994792 的交叉比例；改为逐项居中滚动，保留 `ratio: 1` 完整可见断言和真实点击准入检查，不降低阈值。900×600 的编程推荐复验已通过。
- SPEC 已更新编程四项集合、单用途并集覆盖与小窗口检查要求；不改原生状态或原任务历史材料。
- 首次引导全尺寸 Chromium/WebKit 复验 30/30 通过。
- `TRELLIS_CONTEXT_ID=fyagent-grok-guide-correction mise run check:prearchive --exclude-active-task .trellis/tasks/09-15-grok-guide-recommendation` 完整退出码 0，覆盖类型、Lint、格式、前端/契约单测、桌面 mock、Rust 格式/check/Clippy/测试和发布契约；发布子集另显示 619 通过、1 项既有跳过，native-fetch 4/4 通过。不把重叠子集累计为新增测试量。
- 最终 `mise run build:renderer` 通过，八个路由独立分包；初始 JS 665004 字节、CSS 46664 字节，与本次修复前一致，未调高预算。
- 最终生产 `production boots` 3/3 通过，覆盖八路由初始化、引导按需加载/跳过和动效时间单位。
- 最终目录、模型和引导浏览器回归 90/90 通过，四档 Chromium 窗口与 WebKit；包含之前 30 项引导测试，不重复累计。
- 最终差异复核仅推荐数据、测试、owner SPEC 和本任务记录；原生能力、首次判定、普通用户数据均未修改。

## Commit plan

一笔 `fix(agents): include Grok Build in first-use recommendations`，包含推荐数据、页面/浏览器回归、owner SPEC 和本任务材料。
仅暂存本任务已知文件，不含其他工作；之后由任务工具生成归档与会话记录提交，不推送远端。
归档后运行不带 active-task 排除的 `mise run check:contracts`，结果记录到收尾会话日志。

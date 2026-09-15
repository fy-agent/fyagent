# Design

## Boundaries

在原生目录修改简介源数据，不在 Renderer 截断或替换负面短语。推荐规则只记录当前封闭 Agent ID 的用途关联；名称、能力和安装状态仍由原生目录与既有生命周期模块拥有。

## First-use authority

复用本机 `settings.json` 的锁和持久化 owner，增加可选 `firstUseGuideState`：`pending | dismissed`。缺省读取为 dismissed（兼容旧安装）。首次初始化只有数据库、旧 config.json 和本机设置均确认不存在时记录 pending；在创建数据库前持久化，避免 seed 后丢失首次身份。已有 pending 保留；旧欢迎确认始终抑制引导。

两个无参数窄命令读取/结束引导；结束只合并引导状态和旧欢迎确认字段，不提交整份前端设置快照。普通 `save_settings` 在既有锁内保留新字段。用途选择只留在页面内存，不写遥测或外部软件。

## Renderer

在 Agents 路由的目录分支内插入两步引导，使用现有 Button、品牌图标和主题 token，不引入依赖或新路由。首次状态为异步查询，尚未明确时不闪出引导；失败不阻塞主目录。显式 target 不拦截。未完成引导时不启动目录扫描。

引导和它的 CSS 使用既有 React lazy/Suspense 机制按需加载，旧用户不加载一次性引导 chunk。新用户由实际提交的引导组件报告 frontend-ready，不能让加载占位提前显示原生窗口；目录失败/空状态独立报告 ready。生产包体积遵循现有 665600 字节上限。

用途选择：办公优先 QoderWork CN、TRAE Work CN、WorkBuddy；编程优先 Codex、Claude Code、OpenCode；混合展示 WorkBuddy 与 Codex，覆盖两类入门路径。结果严格与解析后的目录取交集，沿用目录顺序，不生成安装权限。

完成与跳过都等待原生持久化成功后退出，失败保留当前步骤并给出简短重试提示。推荐页提供重新选择和查看全部软件，避免未安装用户被送入不可配置页面。操作使用同步防重入锁；隐藏/卸载后不写页面状态。

## Research

2026-09-15 核对官方产品定位（用途关联，不作为 FyAgent 能力/安装权限来源）：

- https://docs.qoder.com/ ：QoderWork 桌面工作助手，文件整理、数据处理、文档生成。
- https://www.trae.cn/ ：办公、文档、数据与代码场景。
- https://www.workbuddy.ai/ ：日常办公 AI 工作台。
- https://openai.com/codex/ ：编程任务。
- https://code.claude.com/docs/en/quickstart ：代码理解、修改与测试；本项目只承诺已接入 CLI。
- https://opencode.ai/ ：开源编程 Agent，桌面/终端使用。
- https://support.apple.com/guide/safari/keyboard-shortcuts-and-gestures-cpsh003/mac ：WebKit/Safari 的 Tab 导航受本机 Keyboard Navigation 设置影响；Option-Tab 遍历可点击控件。最小原生按钮复现已证实当前测试主机行为，测试不修改用户系统设置。

## Compatibility and rollback

不改目录 wire 版本、ID、capability、安装按钮策略、数据库 schema 或依赖。新增设置字段为可选；旧版本忽略它。回滚产品提交可移除 UI 与窄命令；保留字段不会影响原配置。不得以本次浏览器 mock 结果声称完成真机首次安装验收。

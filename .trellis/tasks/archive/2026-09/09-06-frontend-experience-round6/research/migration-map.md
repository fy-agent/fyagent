# 单一前端迁移设计与审查边界

目标不是把旧树改名legacy后继续维护两套实现，而是结束代次分叉。保留当前
生产能力与真实安全合同，移除没有生产入口的旧界面及其专属依赖；拆清纯域与
视图运行时。文件迁移、业务修复和依赖清理分提交，便于review及Git相似性识别。

## 目标布局

```text
src/
  index.html              唯一应用HTML入口，不携带离线测试重定向
  main.tsx                当前生产启动
  app/                    组合根、路由、主题编排和样式
  pages/                  既有七条路由
  widgets/                桌面外壳
  shared/                 当前共享UI、features和platform
  domain/                 不依赖React/Tauri的纯协议/配置/投影逻辑
tests/
  renderer/               当前组件、交互和适配器测试
  browser/                同一产品的浏览器集成与性能测试
  domain/                 保留下来的纯域/配置/解析安全测试
  ...                     现有工具链、原生合同和发布测试按职责保留
```

`src/shared/codex-desktop`的7个纯域模块迁至`src/domain/codex-desktop`；当前
renderer的同名UI模块仍在`src/shared/codex-desktop`，不得覆盖两者之一。
旧树中确有独立使用价值的纯逻辑按其业务职责迁入domain；没有现行消费者及
安全合同的旧组件/旧App/bootstrap退出。禁止通过旧路径re-export永久维持分叉。

## 可执行入口映射

| 现有                                              | 目标处理                                                                                    |
| ------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| `src/v2/**`                                       | 迁到src中对应角色目录；修正别名、相对import、URL、原生资产引用                              |
| `tests/v2/**` / `tests/v2-browser/**`             | `tests/renderer/**` / `tests/browser/**`                                                    |
| `tsconfig.v2.json` + `tsconfig.json`              | 一个应用/测试strict入口；Node构建配置可保留`tsconfig.node.json`，它是环境边界不是版本分叉   |
| `eslint.v2.config.mjs`                            | `eslint.config.mjs`；统一renderer/测试作用域和禁止越层规则                                  |
| `vitest.v2.config.ts` + `vitest.config.ts`        | 单一Vitest配置，必要时用已安装3.2.7的projects按renderer/contracts划分环境，不按新旧代次分组 |
| `playwright.v2.config.ts`                         | `playwright.config.ts`，同一套产品场景的Chromium/WebKit项目                                 |
| `playwright.v2-performance.config.ts`             | `playwright.performance.config.ts`，生产、串行、固定视口的性能目的配置                      |
| `scripts/verify-v2-route-chunks.mjs`              | `scripts/verify-route-chunks.mjs`，原分包与启动预算不变                                     |
| `build-v2-preview`、声明及专属HTML解析组合        | 删除离线分发功能；有真实仍用消费者的通用能力才保留                                          |
| `test:v2`/`lint:v2`/`typecheck:v2`及watch/browser | 合并到`test:unit`/`lint`/`typecheck`/`test:browser`等职责命令，不留同义版本别名             |
| `.trellis/spec/frontend/v2-*.md`                  | 依实际主题去掉代次前缀；兼容路由文档引用合并到现行最小owner                                 |
| 测试或代码`agents-v3`等renderer代次名称           | 改为行为名，如agent-directory/agent-auth；不是直接替换所有v3字符串                          |

必须同步`package.json`、mise任务/生成任务文档、CI变更分类与依赖、根合同测试、
Tauri allowlist中真实文件路径、README/开发手册、dependency-cruiser、资产与结构
摘要、测试setup/fixture/build输入。新的聚合check必须实际包含当前renderer质量
检查，不再把当前产品测试留成另一个容易漏跑的入口。

## 退役清单的生成规则

实现前从真实HTML/生产入口、脚本入口和测试入口各自建立可达图；对每个删除
组记录旧路径、消费者、原合同、目标测试和退役理由。禁止把“当前入口不可达”
等同“安全测试无价值”。带UI的旧深链接测试迁入现行接收/校验边界或已有Rust
安全用例；仅对HTML生成器自身的测试才随入口退出。任何无法证明保留等价行为
的安全职责先迁移再删除，不留另一套可启动旧App。

旧主题提供者已有`fyagent-theme`偏好键及`set_window_theme`边界：后续主题任务
复用其有效合同并在当前app/platform实现，不删除用户偏好或另造并行主题owner。
依赖卸载以运行时+测试+构建三个入口无使用为条件；不凭名称删除i18n、编辑器、
schema或Tailwind包。记录lock差异和审计，不整树升级基础框架。

## Git / SPEC引用

使用工具的Move操作或标准Git可追踪的等价移动，再同步有效引用；内容改写与
移动尽量分开，检查`git diff --find-renames`。Git不存一个需手改的“rename引用”。
不改旧提交、不批量重写历史任务标题/工作哈希。归档jsonl等仍被工具读取的
上下文路径随SPEC移动更新；历史叙述保留，并由迁移映射说明旧位置。
真实API路径、数据库/schema版本、第三方软件版本、Rust DTO中的catalog v5等
都不是前端V2债务，不允许regex全仓把数字删掉。

## 删除HTML后的验证替代

保留`src/index.html`和常规Vite build/preview开发服务器；删除用户下载离线
生成器、根单文件预览、file:重定向及对应特殊构建模式。使用Playwright的
loopback server和现行IPC fixture自动验证应用，不新增用户测试页面或托管生成器。
深链接协议、非法参数/编码/执行风险校验仍由已有Rust/领域测试负责。

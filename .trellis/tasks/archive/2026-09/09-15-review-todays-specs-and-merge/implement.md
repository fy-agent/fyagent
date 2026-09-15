# Implementation and validation plan

## Plan

- [x] 复核六个提交及相关实现、测试、SPEC 和索引，记录 owner 与证据边界。
- [x] 收紧普通设置合并：原样保留两个首次引导原生字段，并添加双向防覆盖回归。
- [x] 将推荐说明改为封闭目录身份的穷尽映射，移除通用简介兜底。
- [x] 收敛后端/前端 first-use owner SPEC 和必要的相邻发现入口；不改历史归档任务。
- [x] 运行格式、聚焦 Rust/Renderer 测试、SPEC/task context 校验和完整归档前门禁。
- [x] 复核最终 diff、包体/浏览器证据需要与未验证边界，提交产品与任务变更。

归档后的 GitHub 交付按治理顺序执行：先运行归档后契约检查并确认工作树、base
drift 与精确 head，再推送分支、创建 PR、启用 exact-head auto-merge，并跟进 PR
及 Merge Queue CI，直至读回最终 `main` merge SHA。动态 PR/run/SHA 证据不反写
已经归档的稳定任务记录。

## Focused validation

```bash
mise run format:check
mise run test:unit -- tests/renderer/platform/firstUseGuidePort.test.ts tests/renderer/pages/agents/Page.test.tsx
cargo test --manifest-path src-tauri/Cargo.toml save_settings_preserves_native_first_use_fields_in_both_directions -- --nocapture
python ./.trellis/scripts/task.py validate .trellis/tasks/09-15-review-todays-specs-and-merge
```

具体 Rust 测试过滤器以修改后的测试名为准。完整准入使用：

```bash
TRELLIS_CONTEXT_ID=fyagent-review-todays-specs \
  mise run check:prearchive \
  --exclude-active-task .trellis/tasks/09-15-review-todays-specs-and-merge
```

归档后执行：

```bash
mise run check:contracts
```

## Review gates

- 普通设置保存不能改变任一首次引导原生字段。
- 新目录身份不能在缺少用途说明时通过类型检查。
- SPEC 只保留稳定契约，不记录本次 run ID、提交 SHA 或重复历史测试数字。
- 浏览器夹具、macOS 宿主测试和 GitHub CI 各自只支持对应证据层级。
- 推送后的 PR head 必须等于本地已复核 head；任何新提交都使旧 auto-merge handoff
  失效并要求更新 exact-head guard。

## Validation evidence

- 聚焦 Renderer/端口回归：2 个文件、47 项通过。
- 聚焦 Rust 回归：普通设置双向防覆盖 1 项、首次状态模块 5 项通过。
- `check:prearchive` 退出码为 0：188 个 Vitest 文件中 1680 项通过、1 项跳过；
  Rust 核心 3246 项通过、5 项按声明忽略，后续集成/辅助组件套件均无失败；桌面
  mock、视觉基线预检、格式、类型、Lint、Clippy、release、平台和任务合同均通过。
- `test:browser` 退出码为 0：生产 Renderer 构建和 route chunk 校验通过，首次引导
  保持独立的 3.42 kB JS / 1.64 kB CSS chunk；生产启动 3 项、完整 Chromium/WebKit
  矩阵 616 项全部通过。
- 浏览器测试使用受控 Tauri 夹具，只证明 Renderer 行为和生产分包；本次没有声称
  完成 macOS/Windows 物理全新安装、安装器、签名或原生凭据存储 HIL 验收。

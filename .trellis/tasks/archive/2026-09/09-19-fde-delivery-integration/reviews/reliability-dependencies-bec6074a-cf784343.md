# 依赖阶段独立审查

审查对象：固定提交 `bec6074a`（Vitest 与 AbortSignal 兼容）和 `cf784343`（Cargo 插件锁文件）。产品、测试、manifest、lock 均从固定 Git 对象读取；历史告警参考 `~/Documents/FyAgent-0.4.5-本机验收-20260919/dependency-alerts.md`。任务内 dependency-implementation / dependency-upgrade-evidence 仅作为实现线的检查记录，不替代独立运行证据。

## 结论

**未发现这两个提交新增的阻断问题，可集成。** 固定 manifest/lock 已移除已知 Vitest 3.2.7、@vitest/mocker 3.2.7 和 rand 0.7.3 路径；AbortSignal 改动没有替换 Request/fetch、吞取消或关闭警告。Linux glib 告警仍未修复，不能报告为五项告警全部清零或 GitHub 默认分支已关闭。

## 核对结果

### 1. 版本与锁文件一致

- `package.json:60` 为 `^4.1.11`；pnpm importer 的 specifier 与解析版本一致，`pnpm-lock.yaml:1393、2617` 分别锁定 mocker / vitest 4.1.11。固定锁内没有旧 3.2.7 节点或引用；Vitest 子包版本同时更新，未用 override 强塞不同代子包。
- `.node-version` 固定 24.19.0，满足锁文件声明的 Vitest 4 Node 范围；现有 Vite 7.3.6 也满足 peer 范围。没有为了升级改动或隐藏运行时版本检查。
- `Cargo.lock:5329、5384` 将 tauri-plugin 更新到 2.6.3、tauri-plugin-fs 到 2.5.2；Tauri 主包 2.11.1 与 tauri-utils 2.9.3 未升级。Cargo.toml 既有 Tauri 2.x 约束保持有效，没有人为增加 rand 直接依赖或不兼容 patch。
- 固定 lock 中 kuchikiki 0.8.8-speedreader、selectors 0.24.0、phf_generator 0.8.0 与 rand 0.7.3 一起消失，现存 rand 为 0.8.8 / 0.9.5 / 0.10.2（`:4048-4070`），与既有 advisory 记录的修复分支相符。独立解析前后 package blocks，并逐一核对固定 lock 内 dependency 名称/版本引用，未发现缺失或多义边；这是静态 lock 完整性核对，不冒充 Cargo 构建。

### 2. 测试收集契约未被绕开

`config/vitest.config.ts:29-75` 保留 contracts / host-integration / renderer 三个 projects，原 include/exclude 和 setupFiles 未改；renderer 仍属于普通 unit 入口，Playwright 仍独立。`package.json:21-26` 保留 `--throw-deprecation`，测试没有新增 skip、降低断言、关闭错误或扩大排除列表。coverage 仍只有 reporter 配置，没有此次升级新增的覆盖集承诺。

实现线记录了跨三个项目的定向实际执行（57 passed / 1 平台 skip，以及另一组 35 passed）；本审查没有重跑这些检查，也不据此推导整个固定快照全库通过。root 的统一检查应继续使用同一 config，确认三个项目按原契约收集即可，无需再造一个 runner。

### 3. AbortSignal 行为与清理

- `tests/renderer/app/setup.ts:47-74` 移除重复 realm bridge，只对已经 aborted 的真实 AbortSignal 阻止新增 listener；活动 signal、普通 options 和非 signal 对象仍进入 Vitest/jsdom 原来的监听路径。没有伪造“取消成功”或 catch 后忽略校验错误。
- `abortSignalRealm.test.ts:4-28` 验证活动信号取消后 DOM 和 window listener 不再触发，原生 Request 也进入 aborted；`:31-50` 验证同一信号覆盖多个 DOM 目标，并保持 Request.reason 的对象身份；`:53-62` 补充了预先取消信号在 button 和 window 两个入口均不注册。
- `setup.ts:39-44` 在 afterAll 恢复 prototype/window 方法；`:76-78` 保留 React cleanup。取消后的 listener 不再触发有行为断言。未看到本地新增的长期 abort listener 或缓存需要另一套清理机制；对 Vitest 内部实现的内存回收不作额外保证。
- `setup.ts:17-36` 的 act warning 失败保护保持原样，其他 console.error 继续传递；原生 fetch/Request 的 setupGlobals 检查保持原样。没有用压警告完成兼容。

## 必要残留与平台边界

1. **glib 保持 OPEN。** `Cargo.lock:2048-2049` 仍为 0.18.5。已有研究记录 Linux GTK/WebKit 正常运行依赖、gtk ^0.18 / Tauri 的解析约束；这两个提交没有隐藏或修复它。不应以 macOS 不加载该链或 npm audit 为零关闭该问题。维持既定上游兼容升级后续即可，不要求此次硬推图形栈大版本。
2. **插件兼容验证目前主要是当前 macOS host。** 实现线记录 canonical locked rust:check 成功，未证明 Windows/Linux 构建或原生 UI。此次 lock 除去旧 HTML 构建链外也重解了部分 Windows 绑定及引入 plugin-fs 的 objc2-foundation 路径，不能只凭 semver 宣称所有平台已验收。沿用现有 Windows/Linux CI 的 locked 构建；在集成原生 smoke 中复用文件选择/保存与应用启动检查，不另建平台框架。没有源码证据证明已出现可见平台回归。
3. **安全结论有限。** 实现线记录 pnpm audit 为零；cargo-audit 安装中止，没有完整 RustSec 数据库扫描结果。本审查只确认上述已知 advisory 对应版本/路径变化，不声称整个 Cargo lock 无漏洞或默认分支告警关闭。

## 执行边界

本次仅固定源码/锁文件静态审查及小型只读 lock 引用解析，未安装依赖、未运行全库、未调用外网、未修改产品。产品路径相对于 `~/.codex/worktrees/fyagent-fde-reliability/fyagent` 的固定提交。无必须返修项。

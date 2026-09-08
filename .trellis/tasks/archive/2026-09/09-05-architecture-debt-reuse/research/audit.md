# 架构与复用审查（2026-09-05）

## 基线与方法

基线 d628f53b。检查范围：`src`（含 V2、leftover、neutral）、`src-tauri/src`、`src-tauri/user-helper/src`、`scripts`，另对现有架构/功能测试、依赖清单、SPEC 与任务入口交叉检查。全量命令：

```sh
mise exec -- pnpm dlx jscpd@4.2.5 src src-tauri/src src-tauri/user-helper/src scripts --min-lines 20 --min-tokens 150 --max-lines 100000 --max-size 5242880 --format rust,typescript,tsx,javascript --reporters json --output /tmp/fyagent-architecture-full-before --silent
```

结果：1,074 files / 442,302 lines / 79 clones / 3,096 duplicated lines (0.7%)。包括内联 Rust 测试，不包括独立 tests 目录。初次默认 max-lines 扫描遗漏大文件，已废弃初次统计。精确重复扫描只用于候选发现，不证明语义等价。工具固定版本，仅临时执行，不添加产品运行依赖。

## 第一轮：候选与处置

| 范围                        | 证据/风险                                                                     | 处置                                                             |
| --------------------------- | ----------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| Tooling versions            | 手写 SemVer 解析容许空预发布/前导零，超大数字预发布被当字符串                 | 使用锁图已有 semver，验证官方 precedence                         |
| S3/WebDAV                   | 三段共 168 行重复（含 41 行测试）；DB 更新钩子直接调用两厂商服务              | 一个 Tokio 调度拥有者，注入通知，传输策略保留                    |
| MCP Qoder/TRAE/WorkBuddy    | 读取、备份、metadata stripping 和投影重复                                     | 一个内部 JSON 文档适配器；不合并产品路径/导入规则                |
| V2 保存与切换计划           | 保存组件 75+38 行重复；三个 interval 未处理慢读取/隐藏                        | 共享保存编排 + 已采用的 Query lifecycle                          |
| Proxy/forwarder/handlers    | 多协议 retry、permit、流式首包、补偿分支相似；最大服务 7,505 行含大量测试     | 保留不同协议/副作用顺序，不用去重器改变安全语义                  |
| Change Plan native service  | 759/1008 两段 130 行相似，有不同资源准备/恢复语义                             | 保留原生事务拥有者，本次仅统一其前端消费者                       |
| Installer / Windows / macOS | 原生特权、身份、helper、签名及回滚边界；已有外部 Blessed/Authorized/SecureXPC | 不更换已复用框架，不把平台差异抽为任意执行器                     |
| 数据库/DAO                  | 仍有领域 DTO/历史迁移依赖与 model_pricing 启动维护                            | 修复具体云同步反向依赖；不机械迁移 schema/DAO 或掩盖剩余依赖     |
| V2 Models/OpenCode/Memory   | 相似 UI 与 reread、revision、overwrite 分支                                   | 保留各自 revision/secret 生命周期；通用视觉控件已有 shared owner |
| leftover forms/quota/query  | 非生产 V2 的重复，引用边界已隔离                                              | 不为删除重复把 V1 引入 V2，后续迁移按实际消费者处理              |
| Repository scripts/tests    | 大型平台/发布门禁与重复测试夹具                                               | 保留独立断言/平台证据，不用减少测试来美化重复率                  |

## 开源依据与选择

- semver API: https://docs.rs/semver/latest/semver/struct.Version.html 。`Version::parse` 校验完整语法；`cmp_precedence` 排除 build metadata，而 Ord 不等同于升级优先级。源码/许可：https://github.com/dtolnay/semver 。使用现有锁定版本，MIT OR Apache-2.0；不是 JS/npm 范围表达式解析器，不用于 MSIX 四段版本。
- Tokio mpsc: https://docs.rs/tokio/latest/tokio/sync/mpsc/index.html ，有界队列及背压；Receiver::recv 是取消安全的。https://docs.rs/tokio/latest/tokio/sync/mpsc/struct.Receiver.html 。只组合已有 mpsc/time，不引入调度框架。
- rusqlite update hook: https://docs.rs/rusqlite/0.31.0/rusqlite/struct.Connection.html ，本地实际依赖 API 需再次编译验证；回调在连接操作期间同步触发，不执行阻塞上传/数据库再入。
- TanStack Query keys/cancellation: https://tanstack.com/query/latest/docs/framework/react/guides/query-keys 、https://tanstack.com/query/latest/docs/framework/react/guides/query-cancellation 。Query 拥有请求共享和读取缓存；业务写入仍是显式用户操作。v5 的 interval 回调接收 Query；以安装版本类型及回归测试为准，文档旧路径可能重定向。
- Tauri 架构：https://v2.tauri.app/concept/architecture/ 、https://v2.tauri.app/develop/calling-rust/ 。保留可信 Rust 宿主与类型化 IPC，不引入任意命令桥。
- jscpd：https://github.com/kucherenko/jscpd 、https://jscpd.dev/getting-started/v4 。采用固定 v4 作为一次性审查工具，非业务依赖。

依赖策略：不升级 React、Tauri、Tokio、rusqlite 或 TLS/HTTP 栈。新增直接引用仅复用现有 semver 锁定节点；Tokio test-util 如需则只启用测试 feature。来源验证不等于证明不存在安全漏洞；本任务不宣称全依赖安全认证。

## 第三轮复扫与约束复核

复扫使用同一命令，仅将输出目录改为 `/tmp/fyagent-architecture-full-after`：

| 指标                         |    基线 |  实现后 |
| ---------------------------- | ------: | ------: |
| 源码文件                     |   1,074 |   1,079 |
| 扫描行（包含 Rust 内联测试） | 442,302 | 442,182 |
| 精确重复块                   |      79 |      70 |
| 重复行                       |   3,096 |   2,668 |
| 重复比例                     |   0.70% |   0.60% |
| 重复 token                   |  27,180 |  22,743 |

重复行减少 428（约 13.8%）。新增私有共享拥有者及测试后，扫描行净减 120；不能把这个数解释为整个 diff 或生产业务代码删减量。剩余 70 处不是全部缺陷，本表也不是全仓语义债务清零证明。扫描没有通过改阈值、删独立夹具、排除大文件美化结果。

已核对原生调用链：命令侧同步 import 的两个 RAII guard 仍分别使用各自 backend；数据库 listener 的注册失败在 worker spawn 前返回；原有上传锁、事件、settings/error store 保留。MCP 适配器保留产品路径、门槛和规范化，JSON handler 保留 pretty/order 与 exact-byte backup。所有对外 Tauri command/DTO/ACL、schema、native compensation 均无变更。

补充验证：semver 直接检查锁定版本 API https://docs.rs/semver/1.0.27/semver/struct.Version.html ，Tokio 使用 https://docs.rs/tokio/1.50.0/tokio/sync/mpsc/index.html 。TanStack GC 行为同时查看安装版本 `query-core/src/removable.ts`、QueryClient 官方文档及失败后转绿的回归用例，不把 latest 文档重定向页面当成兼容性证明。

最低 WebView 复核：`tauri.conf.json` 最低 macOS 为 12.0；MDN 的 `throwIfAborted` 基线为 2022-04，而 `aborted` 为 2018-04。没有必要新增对便利方法的依赖，改用后者及库自身的 `CancelledError`。资料：https://developer.mozilla.org/en-US/docs/Web/API/AbortSignal/throwIfAborted 、https://developer.mozilla.org/en-US/docs/Web/API/AbortSignal/aborted 。这只证明依赖面更保守，不替代最低系统原生 HIL。

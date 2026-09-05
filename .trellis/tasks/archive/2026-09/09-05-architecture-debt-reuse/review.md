# 集成评审与验证记录

## 评审结论

本任务完成的是有仓库级范围的候选审查和四项有明确复用/生命周期收益的架构改造，不是逐行证明全仓不存在技术债。第一轮按 SPEC、依赖与克隆扫描建立候选；第二轮锁定业务/机制边界；第三轮对整个改动、调用方、错误路径、缓存并发及规范漂移进行复核。没有以拆文件或换框架代替修复，也未委托独立外部审计。

## 已验证的结果

| 检查                           | 结果                                                                                                  |
| ------------------------------ | ----------------------------------------------------------------------------------------------------- |
| 起始工作区                     | `dev/laiyongjie` / `d628f53b`，干净                                                                   |
| Trellis context validate/start | 通过，任务 session identity 为 `fyagent-architecture-debt-reuse`                                      |
| 初始 architecture 测试         | 3 文件 / 9 测试通过                                                                                   |
| 当前 architecture 测试         | 3 文件 / 11 测试通过                                                                                  |
| `typecheck:v2` / `lint:v2`     | 通过                                                                                                  |
| 当前 `test:v2`                 | 72 文件 / 511 测试通过                                                                                |
| 通用前端 `test:unit`           | 177 文件 / 1,589 passed / 1 既有 skipped                                                              |
| `test:v2:browser`              | 164 项浏览器回归全部通过，多视口 Chromium                                                             |
| `check:backend`                | 格式、locked check、Clippy deny-warnings、全量 Rust 测试通过                                          |
| Rust 测试汇总                  | 3,469 passed / 0 failed / 6 ignored（既有显式环境测试）                                               |
| `build:renderer`               | 成功；生产 route chunk 验证及 standalone preview 生成成功                                             |
| 同口径 jscpd                   | 79 -> 70 clones；3,096 -> 2,668 duplicated lines                                                      |
| 完整 `check:prearchive`        | 通过（exit 0），精确排除本活动任务；环境、通用前端、Rust、任务/平台/Python/version/release 契约均通过 |

执行日志在本机 `/tmp/fyagent-architecture-{backend,v2,build,boundaries,browser,prearchive}.log`；扫描报告见 research/audit.md 的固定命令及输出目录。这些临时日志不是仓库依赖，不要求它们长期存在才能阅读本记录。

完整门禁中的平台面校验扫描 2,670 个当前文件；发布契约子集 610 passed / 1 既有 skipped，native-fetch 4 passed。这些子集与通用前端存在重叠，不能把所有数字直接相加当作去重测试总数。

## 失败与修复

1. 首次将 `tests/v2` 传给根 `test:unit`，因项目明确隔离 V2 而显示 no test files；改用 `mise run test:v2`，未放宽 Vitest include/exclude。
2. 新增 GC 断言发现种子 Query 使用默认缓存时长，之后的短 GC 无法覆盖。修为 seed 前设置单一 job family default，原断言保留并通过。
3. 多 observer 复核要求隐藏/关闭一个消费者不能误取消另一个；通过 inactive cancellation 和 Query 自身 observer 卸载处理，不另造缓存/计时器。
4. 旧 snapshot seed/读取不能覆盖新 revision；对照原生 DAO 的递增约束增加缓存断言与测试。
5. 注册 listener 必须先于 worker spawn；调整组合根顺序并写回设计。
6. 完整 prearchive 首轮发现 5 个已评审的平台敏感源码摘要漂移（Cargo manifest、lib 组合根、TRAE MCP、Tooling versions、Rust 架构测试）。按照 task-runner identity seal 契约只同步这 5 个摘要，未增加 exclusion、未改 checker 或测试阈值；重新执行整套门禁。

   对应契约为 `.trellis/spec/backend/task-runner-contract.md` 的 `Supported-platform identity seals` 小节；本轮已实际读取。该文件超过任务上下文单文件 32 KiB 上限，检查上下文引用本评审记录及精确节名，而不是注入被截断的全文；未修改全局注入限制或缩减门禁。

7. 最低 macOS 12 WebView 兼容性复查：避免新增对较晚出现的 `AbortSignal.throwIfAborted` 的依赖，改用标准 `aborted` 和已安装 Query 导出的 `CancelledError`；专项测试让新便利方法不可用，仍需通过全部生命周期断言。

## 不变边界与剩余风险

- SemVer 非法输入变为严格拒绝；三段升级优先级与 trim 保留，metadata 不参与比较；MSIX/渠道策略未修改。
- 调度器的错误恢复只处理后续 dirty hint，不凭空增加网络重试。数据库更新 hook 不等于事务提交、队列不是 durable。
- MCP 原文备份失败不覆盖文件；统一 upsert 原本存在的 durable/live 非原子边界仍在，未宣传新增跨文件事务或 symlink 防御。
- Query 仅自动读取脱敏 job snapshot，原生写入及即时 reread 仍显式执行；API key 未进入 query/mutation cache；取消接纳 IPC 结果不等于取消原生执行。
- Proxy 协议、Change Plan native compensation、安装器权限、其他领域 DTO/历史迁移等有独立不变量，保留为后续按领域处理的候选，详见研究处置表。
- 6 个 ignored 为两个备份性能诊断、两个 live S3、真实 Codex corpus、原生 OS credential-store HIL；本任务未修改这些 skip。Mac 测试不证明 Windows 原生运行、安装签名或真实第三方应用重载。
- Standalone preview 保留 Vite 500 kB 提示；未调高告警阈值，生产构建通过不等于所有性能债务消失。

## 交付收口

SPEC 已对齐数据库/同步/MCP/模型生命周期的签名、行为、错误矩阵、断言和复用规则。完整 prearchive 已通过；按工作提交、Trellis 归档、归档后无排除参数 canonical contracts、会话记录的顺序收口。最终 Git 状态、提交哈希与归档位置以任务元数据、会话记录和实际命令输出为准。不推送、不发布、不清理用户数据。

实际工作提交为 `a051c09846a2a600c2fc513335e10e1fc15a93f0`，任务于 2026-09-05 归档，归档提交为 `42acc3a6`。归档后 `mise run check:contracts` 无排除参数执行成功（exit 0），日志 `/tmp/fyagent-architecture-postarchive-contracts.log`。归档脚本没有改写 JSONL 内两个指向任务自身的路径，已在归档记录中将其改为真实 archive 路径并复验上下文；该修正不改源码、全局脚本或历史运行命令。

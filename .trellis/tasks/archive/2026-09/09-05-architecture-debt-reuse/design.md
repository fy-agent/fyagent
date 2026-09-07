# 设计与评审边界

## 决策

采用渐进式模块整理，不替换应用框架、不添加通用插件/工作流/事件总线。四个交付共享同一集成检查和规范更新，使用一个任务而非人为分裂相互关联的验收。

1. Tooling 保留 `compare_semver` 门面，内部调用 `semver::Version::parse(trim)` 和 `cmp_precedence`。已有 Cargo.lock 的 crate 升为直接依赖，不升级锁图。非法 SemVer 返回 None；保留合法版本的原有优先级、trim 行为和 npm channel 策略。Windows MSIX 版本不是 SemVer，不改。
2. `services/auto_sync.rs` 私有拥有调度机制。每个同步后端持有独立 controller；Tokio 有界 mpsc 保留 dirty 信号，Tokio 时间驱动防抖。抑制在 SQLite 通知到达时检查，不能延迟至消费，否则恢复导入会回传。后端保留上传函数、独立互斥锁、设置及事件。`Database::set_change_listener` 仅接收受信任的 crate 内闭包；第三轮评审收紧为组合根先注册回调，再启动两个 worker，避免注册失败后留下已启动任务。回调在连接锁内执行，只允许非阻塞通知，不允许再入 DB 或上传。
3. `mcp/json_document.rs` 私有拥有 JSON mcpServers 文档读取/元数据剥离/备份/写入，复用 config reader、serde_json 和现有 atomic_write。适配器保留路径解析、存在性门槛、WorkBuddy 隐藏文件回退、Qoder 类型归一化和导入合并标志。保留原来的 pretty JSON 键顺序（不切换到会排序所有键的 write_json_file）。不扩大文件写入能力。
4. 保存工作区共用一个 typed 组合组件，包装器只负责创建计划的闭包和文字。Query 只缓存脱敏的 ChangeJobSnapshot，以 jobId 为 key，负责定时读取和 single-flight；本地组件保留一次性写入及请求代际控制。密钥请求不交给 useMutation，因此不会保留在 mutation cache。统一轮询 hook 同时用于切换 Provider 工作区。隐藏/终态/读取错误停止自动查询，当前原生任务不被取消。关闭/卸载使未完成的 UI 请求失效；取消 Query 只取消读取结果的接纳，不声称取消 Tauri 原生执行。

## 兼容性与失败语义评审（第二轮）

- 数据库更新钩子不是 commit hook，回滚仍可能产生 dirty hint；保存原语义，不宣传 exactly-once 或已同步。
- 内存数据库不再默认触发进程全局云同步；生产组合根注册，测试显式注入通知并验证 INSERT/UPDATE/DELETE。
- 防抖达到 10 秒必须 flush；上传期间的后续写入留在容量 1 队列；重复 start 不产生第二 worker。
- 不合并 S3/WebDAV 的传输锁、设置或抑制计数；保留导入既有 RAII API。
- MCP 先验证 root 和所有条目，再备份/写入；JSON 错误、非对象、备份失败都不能覆盖原文件。该重构不宣称新增跨文件事务或 symlink 安全。
- 模型 mutation 不重试；原生 accepted/rejected、consumed、即时 reread 和 terminal callback 去重保持。可见性只控制自动读取，不撤销用户已确认的写入。
- Query retry/focus/reconnect 明确关闭，错误保留上一份 snapshot，不制造成功。测试需覆盖极慢读取、关闭和重复点击。

## 实现反馈（第三轮）

- `setQueryData` 在 observer 之前创建缓存；Query 的 GC 生命周期取历次配置最大值。专项失败测试证明只在 `useQuery` 设置 `gcTime: 0` 不足。改为首次 seed 前设置一个 job family default，而不是每个 job 一个默认项或自制回收计时器。
- 隐藏只取消 inactive 查询；关闭某个 observer 由 Query 管理卸载，不能取消另一活跃 observer 的读取。添加双 observer 回归。
- 原生 `change_jobs` 持久化以递增 revision/event_seq 更新；共享 cache 拒绝较低 revision，避免晚到的旧 seed 覆盖较新 authority。
- 构建保留 standalone preview 的大块体积提示，不为了美化结果调整 warning 阈值；生产 renderer 构建与 route chunk 检查成功。

## 回滚

每项变更保持原有外部门面，可按工作提交 revert。无 schema/格式迁移，无用户数据清理操作。不能为了通过检查放宽架构/类型/安全 gate；新增架构断言限制共享拥有者回流。

# 前端收敛

以父research/migration-map.md为完整迁移合同。先记录当前生产入口、共享域与
工具/测试入口的可达图，建立每组移除/迁移/保留职责映射。移除真正退役UI，
保留仍有价值的安全/解析/配置合同；不要把旧树改名legacy来伪装收敛。

src/v2 → src角色目录；renderer-neutral Codex域 → src/domain/codex-desktop；
tests/v2 → tests/renderer、tests/v2-browser → tests/browser。一个Vitest配置
可用projects隔离renderer与工具/合同环境，一个strict类型入口与一个ESLint。
性能Playwright独立是用途不是版本。构建只产dist，不生成用户离线HTML。

删除deplink.html、单文件预览builder及其file重定向，手册三语言同步。
真实深链接协议/确认/错误/无副作用在Rust与可复用域测试保留，不能靠删测试过关。
同步资产、CI分类、mise、结构快照、现行SPEC与归档jsonl有效路径；不改Git历史。
依赖清理只在生产/测试/工具图证明无人使用时执行，不顺带升级框架。

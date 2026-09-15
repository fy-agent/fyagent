# Design

## Review boundary

本次评审沿完整数据流进行：

```text
本地数据存在性 -> settings 原生状态 -> 无参数 Tauri 命令 -> 严格端口解析
-> query cache -> /agents 路由门禁 -> 两步引导 -> 原生持久化 -> 目录扫描
```

保留两个聚焦 owner：后端 SPEC 负责资格、持久化、命令与普通设置防覆盖；前端
SPEC 负责路由、查询、推荐投影、交互生命周期与可访问性。目录、目录文案、窗口
就绪和 GitHub 治理文档只描述与 owner 的交界，不复制状态机。当前 owner 文档均
足够聚焦且低于上下文限制，因此不为形式拆分。

## Native state authority

`firstUseGuideState` 和 `firstRunNoticeConfirmed` 都视为原生维护字段。专用完成命令
在同一个设置写锁内将二者写为完成值；普通 `save_settings` 无论入参携带什么值，
都从锁内最新设置原样恢复这两个字段。这样同时阻止旧快照重新打开已完成引导，
也阻止任意全量设置调用借旧兼容字段跳过待完成引导。

资格判定继续在数据库创建之前执行。数据库、旧 `config.json` 和设备设置文件三者
必须全部得到“确认不存在”的证据；文件存在、解析失败、权限错误或 `try_exists`
失败均 fail closed。状态写入成功后才更新进程内值，写失败不制造 `pending`。

## Recommendation projection

用途关联继续是页面局部静态集合，并与严格解析后的原生目录取交集。用途说明改为
对封闭 `AgentCatalogId` 的穷尽 `Record`，移除目录简介兜底。未来新增目录身份时，
类型检查会要求明确说明；是否加入某用途仍由覆盖回归和产品评审决定。

不改变当前推荐集合、目录名称来源、排序或任何安装/配置权限。

## SPEC convergence

- 后端 owner 补充双字段写权限、存在性失败矩阵、双向 stale-save 回归和唯一合法
  状态转换。
- 前端 owner 按路由/启动、推荐投影、完成生命周期分组，补充无有效目录时不得
  写确认以及说明映射必须穷尽、不得兜底。
- catalog 和 user-facing-copy 文档保留正向摘要与真实运行错误的边界，不承载首次
  状态。
- 一次性提交 SHA、测试数量、CI run ID 与最终 PR 证据只写本任务，不写稳定 SPEC。

## Git and delivery

`origin/main` 当前只比本分支多上一轮 PR 的 merge commit；该 merge commit 已包含
本分支共同祖先，不需要为了“保持最新”重写六个已评审提交。Merge Queue 是最新
base 集成权威。完成本地生命周期后推送当前分支，以精确 head guard 开启自动合并；
若 PR 或 merge-group CI 暴露问题，在同一分支最小修复、重跑适用门禁并更新精确
head，再重新进入队列。

## Rollback

本次代码修正仅收紧普通设置写权限与推荐说明类型，不改变持久化格式或 IPC 签名。
回滚对应提交即可恢复原行为；已有 `pending` / `dismissed` 文件仍可被前后版本读取。

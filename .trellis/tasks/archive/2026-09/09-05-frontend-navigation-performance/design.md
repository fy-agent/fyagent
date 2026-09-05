# 设计与调查分支

性能测量与采样属于测试工具，不给每个产品组件插入永久console计时。使用现有Playwright夹具和生产renderer，Chrome Performance/longtask/performance marks用于对比；网络和CPU条件显式固定。根因记录区分模块加载、React render、layout/paint与业务等待。

优先检查 `PersistentPrimaryOutlet`、`PersistentSurface`、`usePersistentSearchParams`、SideNavigation 和实际热路径页面。只在证据支持时加入稳定元素/memo边界、独立业务组件或Router支持的可中断transition。共享feature query继续拥有缓存和失效，禁止创建新view store。

预取复用 `primaryPageLoaders`：若首访请求显示竞争或入口意图不足，再通过这个owner调整预热顺序与用户意图；没有证据时不添加悬停下载队列。冷启动仍以初始page及本地图标为就绪条件，不恢复全页面同时阻塞启动。

若大Page中单个重型panel是瓶颈，按真实功能拆到具名文件，再同步导入、测试和spec；仅降低行数不构成验收收益。比较长列表前先保留分页，确需窗口化时复用已装TanStack Virtual，不自己写虚拟滚动。

回滚以独立性能工作提交为单位。设计选择不得改变dirty blockers、隐藏上下文或秘密保存时长；出现这些变化需回到父任务评审。

# 客户项目移除设计

## 删除原则

按真实依赖移除整条功能链：导航与路由 → 页面与状态 → 前端接口 → 原生命令 → 专属服务与数据类型 → 运行时存储引用。不能仅隐藏菜单，不能用 feature flag 留下休眠模块，也不新增替代记录板。

以本任务 research/module-scope.md 的实际分支和文件清单为实施依据。当前规划宿主目录存在其他人的未提交修改，禁止覆盖、stash 或 reset；实施必须先核对包含该模块的正确基线，使用独立工作树并记录 commit。

## 共享边界

项目绑定的账号、模型、Skills、MCP、提示词、记忆，以及用户实际文件/目录，默认属于独立共享资产。删除绑定与模块专属胶水代码；只有确认无其他调用者的实现才连同删除。不得以 project/workspace 等泛词全局批量删除代码。

## 具体删除清单

以集成版本 codex/fde-delivery-integration（590f6f96）作为范围参照，实施基线需再次与最新交付分支对齐：

- 前端：src/pages/projects/、src/domain/projects/、src/shared/features/projects.ts、src/shared/platform/tauri/feature-ports/projects.ts，以及 navigation、primaryPages、ProjectsWorkspace、FeaturePorts、browser/native port 聚合和 query keys 中的接线。
- 子流程：ProjectDeliveryKitsPanel、VerificationPanel 及项目专属导出/交接流程。检查 delivery-kits 与 verification 的 domain、port、服务、命令、DAO、ACL、资源与测试；没有项目外现存调用者的部分整体删除，不因命名通用而留下死模块。
- 后端：commands/projects.rs、services/projects/、database/dao/projects.rs、projects.toml 权限及 capability、lib.rs/service/command/dao 注册。
- 存储：基础实现四张表 fde_customers、fde_projects、fde_project_kit_intents、fde_project_context_versions；集成版 verification 相关表另按真实 schema 补齐。移除专属建表/初始化及 DAO，检查同步 skip/preserve、SQL import/export 与旧版迁移路径。
- 资料与测试：projects 页面/平台/路由/ACL/浏览器测试、项目后端测试、backup 相关断言、现行 4.7-fde-projects 手册及目录。共享回归测试保留并更新。

## 旧数据和兼容

模块完全退出产品和运行时。既有专属数据文件作为历史数据原地保留，新版本不再读写，也不新建目录；不为此保留旧模块接口或新增归档页面。若实际存储与共享存储混合，只停止消费专属字段，不能粗暴删除整库或共用配置文件。

具体选择：默认允许旧数据库中退役表作为不再消费的历史数据留存，不保留其 DAO、业务服务或专用同步恢复逻辑；新安装不创建退役表。若 SQLite 升级链技术上必须重建/删除这些表，先保存可恢复副本，备份失败则停止破坏性步骤。不要重编号既有 schema；下一 schema 号以最终基线为准。研究文档提出的默认物理删表、删目录和另加导出确认建议，在本方案中已被此保守策略替代。

旧路由/上次页面恢复状态通过现存通用默认路由回退，不保留模块展示。旧备份导入需明确忽略退役字段或给出不支持的结果，同时确保共享配置可恢复且不会重新生成模块内容。实施前先核对真实备份合同，若不包含该模块则无需加兼容代码。

## 文档处理

更新现行手册、页面结构说明、翻译、截图和相关规范。旧任务、研究和发布记录保留历史真实性；通过新变更记录明确过去的客户项目设计已被本次决定取代。不得重写 Git 历史。

## 回退

代码回退依赖实施前基线和独立提交；既有项目数据保留使旧版仍有恢复可能，但必须按真实格式验证，不能预先声称已验证。此任务不自动安装旧版、发布或销毁数据。

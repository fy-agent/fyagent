# 集成架构与边界（规划中）

## 当前约束

沿用现有单renderer分层，Domain保持纯类型/解析，shared/platform/tauri拥有原生命令与解码，command薄适配，service拥有业务副作用，Database DAO持有唯一SQLite连接。保留SecretRef边界、reversible-config、ChangePlan和真实结果回读。

## 总控所有权

主控拥有跨任务接口裁决、主导航/路由和lazy注册、统一IPC/ACL接线、schema版本协调、集成测试与最终本机安装验收。四个writer分别拥有自己的feature目录、测试和spec，共享文件须先在交付说明中列出。

## 待首轮方案收敛

项目是FDE新能力的业务入口；交付包与验证/交接以projectId关联。客户项目配置与现有全局配置分开，不能通过切全局配置伪造项目隔离。三类新增域以窄typed接口相互消费，不相互读写内部表。身份、目录、凭据引用、资源快照、revision/fingerprint为首批冻结对象。

隔离描述必须具体到数据/配置/工作目录；本机普通用户进程的文件访问能力不能被UI文案夸大。验证状态分层且独立，所有人工确认与机器观察有来源，样本和客户验收不能自动推导。

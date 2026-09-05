# 边界

在视觉子任务后执行。安装保留目录生命周期 owner。将实际已有的 Codex Provider 切换工作区与通用 apply 视图提升到共享 feature owner，账号页组合现有 Ports；模型保存工作区复用相同底层视图，不制造页面间反向依赖或第二套 OAuth。

官方账号绑定与 Provider 切换仍是不同原生协议，界面统一入口不等于合并存储或更改秘密生命周期。保持回读、stale 再生成、pending 防重入和返回上下文；不能把模型保存暗改为纯草稿保存。

集成选择：共享 ApplyWorkspace/view-model/job observer/errors 移至 `shared/features/change-plans-ui`，Codex/WorkBuddy保存编排仍在Models。ConnectionsView保留同一详情容器与一个隐藏但不销毁的Codex来源面板，避免切视图丢失执行中操作。账号页与来源操作互斥；来源终态必须回读overview和provider summary；读取失败保留结果与重试入口，不直接解除写入保护。此轮不将Claude/厂商交接强行纳入managed账号。

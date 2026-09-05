# 执行顺序

- [x] 核对干净基线、现有第三轮结果、官方资料与已安装开源依赖。
- [x] 形成父任务和三个子任务的需求、设计、验收与上下文。
- [x] 用户确认最终规划后，按顺序 task.py start 对应子任务。
- [x] 完成导航性能子任务；记录生产基线/改后数据，SPEC、工作提交、归档。
- [x] 完成材质和容器响应子任务；图像与对比度矩阵、SPEC、工作提交、归档。
- [x] 完成动效子任务；有源/无源、开关竞争、可访问性和帧性能，SPEC、提交、归档。
- [x] 启动父任务整合检查；审查完整diff及命名/引用映射，复跑性能与组合态UI。
- [x] 运行 mise run typecheck:v2、lint:v2、test:v2、test:v2:browser、build:renderer 及项目完整 check:prearchive；使用runner规定的精确任务排除，不新增豁免。
- [ ] 更新所属 SPEC 后提交并归档父任务；修复因目录移动失效的任务上下文、填写全部工作提交。
- [ ] 归档后无排除 mise run check:contracts、全部任务 validate、提交对象和SPEC链接复核；记录journal，确认无活动任务、无未跟踪或未提交变更。

每一步读取当前 applicable skill/spec；所有环境走mise。原始截图/trace只使用受控夹具，记录实际路径、构建模式、数据及设备；不保存真实凭证，不宣称没有运行的Windows/macOS原生首帧验证。

# 实施与关闭顺序

- [x] 读取当前提交、SPEC、实际桌面时序实现及成熟方案官方资料。
- [x] 创建父任务及两个可独立验收子任务，完成需求/设计/研究/上下文。
- [x] 使用现有生产测试采集未更改产品代码的导航基线。
- [x] 最终规划已批准，两个子任务沿同一TRELLIS_CONTEXT_ID依序实施。
- [x] 材质任务完成图像/对比度/容器回归，更新SPEC后提交097dd80f并归档。
- [x] 动效任务完成实际来源/时序/反转/取消与秘密清理，更新SPEC后提交04485a94并归档。
- [x] 父任务已启动，完成组合态、命名/引用、材质和动效安全交叉复核。
- [x] 实现冻结快照的V2类型/lint/578项单测、244项浏览器及Renderer/生产分包构建通过；源文件指纹无漂移。
- [x] 三组1x/4x各42回访、20暖态模态和冷态/清理检查通过；保留资源竞争导致的失败样本及隔离复测，不扩大预算。
- [x] 最终V2检查及 `check:prearchive --exclude-active-task .trellis/tasks/09-06-frontend-experience-round5` 退出0，只排除直接绑定的本任务。
- [x] 各所属SPEC/签名/测试已在工作提交中更新，集成复核完成；父工作提交及归档由关闭命令确认。
- [x] 归档目录上下文已修正，三个完整工作commit已填写；三个task.py validate及Git祖先关系检查通过。
- [x] 无排除 `mise run check:contracts` 退出0；记录提交、journal和空工作树作为最终关闭命令后置条件，失败时不报告交付。

每个实现阶段都先加载trellis-before-dev，再按trellis-check、trellis-update-spec、
Phase3.4提交和trellis-finish-work收尾。手工文件修改使用补丁，环境/脚本走mise。
长期基线与摘要存任务研究；原始浏览器夹具trace在忽略目录，不提交真实账号信息。

# 实施与关闭顺序

- [x] 读取当前提交、SPEC、实际桌面时序实现及成熟方案官方资料。
- [x] 创建父任务及两个可独立验收子任务，完成需求/设计/研究/上下文。
- [x] 使用现有生产测试采集未更改产品代码的导航基线。
- [ ] 最终规划摘要获确认后，按顺序启动子任务；每条命令沿用本会话TRELLIS_CONTEXT_ID。
- [ ] 材质子任务：先固化颜色/材质/模态帧性能对照，再修改、回归、更新所属SPEC、提交、归档。
- [ ] 动效子任务：复用实际阶段时序，验证来源/目标/内容和返回；更新SPEC、提交、归档。
- [ ] 启动父任务，审查完整diff、主题状态矩阵、命名/引用、材质与动效组合及秘密生命周期。
- [ ] 复跑 `mise run typecheck:v2`、`mise run lint:v2`、`mise run test:v2`、`mise run test:v2:browser`、`mise run build:renderer`。
- [ ] 以同一生产构建/CPU/夹具方法复测导航及模态暖态20次循环、冷访、残留资源；性能退步须解释并优化。
- [ ] 执行 `mise run check:prearchive --exclude-active-task .trellis/tasks/09-06-frontend-experience-round5`，只排除直接绑定的当前任务。
- [ ] 更新父任务涉及的SPEC/索引和集成记录，提交工作后归档父任务。
- [ ] 修复归档目录改变造成的有效上下文链接，填写完整工作commit；对三个任务运行task.py validate及祖先关系检查。
- [ ] 无排除 `mise run check:contracts`；只提交本轮记录，add_session.py记录journal，最后验证无活动任务、工作树干净。

每个实现阶段都先加载trellis-before-dev，再按trellis-check、trellis-update-spec、
Phase3.4提交和trellis-finish-work收尾。手工文件修改使用补丁，环境/脚本走mise。
长期基线与摘要存任务研究；原始浏览器夹具trace在忽略目录，不提交真实账号信息。

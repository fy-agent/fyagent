# 执行与收尾

- [x] 记录官方资料、实际界面基线和问题。
- [x] 完成视觉子任务，更新规范并归档。
- [x] 完成入口职责子任务，更新规范并归档。
- [x] 完成启动子任务，更新规范并归档。
- [x] 完成跨页复核、焦点回归和完整 `check:prearchive`。
- [x] 归档父任务，补全四任务的工作提交与上下文引用。
- [x] 将无排除 contracts、会话记录及干净工作树核验纳入最终收尾。

收尾命令依次执行 `mise run check:contracts`、四任务 `task.py validate`、任务证据提交、`add_session.py` 与 `git status --porcelain`。任一失败即停止，不以归档状态替代验证。

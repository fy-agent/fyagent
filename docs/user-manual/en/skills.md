# Skills

## Install or import

Search in 「发现」, review the Skill, choose a target and confirm. Check 「已安装」 afterward. 「更多 → 从 ZIP 安装」 selects a file before target confirmation.

For 「更多 → 导入本地 Skill」, select unmanaged rows, adjust software switches, then use 「预览导入」 and 「确认导入」 after reviewing the rows and targets. The original source remains in place; linked sources can be read and copied to ordinary directories.

## Assign and recover

Use a target switch for one Skill. 「批量分配」 lets you filter/select resources, choose one target and enable/disable intent, then preview and confirm. Completion requires saved-state readback. If observations changed, preview again. A failed row may have partial writes: refresh actual state before selecting unfinished rows for a new preview.

An import marked 「已入库」 belongs in 「已安装」 for assignment repair, not another import. If insertion is unconfirmed, use 「刷新并查看已安装」 first. Retry only unfinished sources confirmed to remain unmanaged, through a fresh preview.

Details identify linked sources and read-only targets. Linked directories, including linked parents, are not modified or deleted. Linked target switches are disabled; ordinary targets remain available. If update/uninstall is refused, preserve the original and use ordinary directories. FyAgent readback does not prove that another app loaded the Skill; reload and check in that app.

## Updates, backups and storage

「检查更新」 offers individual updates or 「更新全部」. Bulk updates can partially succeed; inspect counts and handle failed items. Read uninstall confirmation; a backup is not guaranteed for every removal. 「更多 → 备份恢复」 restores only an existing listed backup: select a target, then 「恢复」.

「更多 → Skill 设置」 selects automatic, symlink or copy projection. Confirm storage migration. Link protection rejects migration before changes; ordinary I/O failures may produce partial per-item results. Default storage is `~/.fyagent/skills/`, unified storage `~/.agents/skills/`, and uninstall backups `~/.fyagent/skill-backups/`.

[Back to manual](README.md)

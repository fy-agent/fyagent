# Journal - pythonrust (Part 2)

> Continuation from `journal-1.md` (archived at ~2000 lines)
> Started: 2026-09-02

---



## Session 62: Windows vendor installer handoff
<!-- trellis-session: v=2 fp=39c31055b13cbd37 -->

**Date**: 2026-09-02
**Task**: Windows vendor installer handoff
**Branch**: `dev/laiyongjie`

### Summary

Windows Qoder/TRAE/WorkBuddy 一点安装在 ShellExecute 成功后交接官方窗口并结束 job；成功交接保留 PackageBridge EXE；目录文案不再声称已安装。

### Main Changes

- Helper 使用 SEE_MASK_NO_CONSOLE 加固定 open 动词启动官方 EXE，不等待向导退出码
- x64/User-x64 官方包允许 PE32 i386 NSIS stub 仅作为安装器准入
- 成功交接不删除 PackageBridge EXE leaf；失败/取消/MSIX 仍立即 cleanup
- Uninstall InstallLocation 为空时从 UninstallString/DisplayIcon 父目录恢复 INSTDIR
- 目录卡显示官方安装窗口已打开的成功反馈，不把 handoff 画成已安装

### Git Commits

| Hash | Message |
|------|---------|
| `780b5eb8` | fix(windows): hand off vendor EXE install after ShellExecute |
| `d1152999` | fix(v2): show vendor-wizard handoff copy instead of installed proof |
| `e533e848` | docs(spec): record Windows vendor-installer handoff contracts |

### Testing

- [OK] mise run lint:v2 与 typecheck:v2 通过
- [OK] vitest helper 合同 21、V2 agents 61、core/card 50 通过
- [OK] cargo nsis_pe32 stub 与 vendor_exe_success retain leaf 通过；rust:clippy -D warnings 通过
- [OK] supported-platform 表面检查 2510 files 通过；完整 check:prearchive 被 3 个与本任务无关的 Windows 宿主单测挡住

### Status

[OK] **Completed**

### Next Steps

- Windows 原生 HIL：真实官方窗口、UAC 取消、安装完成后库存回读
- 未推送远程

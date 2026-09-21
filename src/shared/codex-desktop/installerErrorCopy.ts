import type { InstallerErrorDto } from "@/domain/codex-desktop";

export function codexInstallerErrorCopy(error: InstallerErrorDto): string {
  if (error.details.platformErrorCode === "target_permission_denied") {
    return "安装目录不可写。请检查目录权限后重试，或将现有 Codex Desktop 移至 ~/Applications 后刷新。";
  }
  if (error.details.platformErrorCode === "disk_space_unavailable") {
    return "无法读取安装目录的可用空间。请检查磁盘是否已连接、目录是否可访问，然后重试。";
  }
  switch (error.code) {
    case "METADATA_CHANGED":
      return "版本或安装位置已变化，请刷新后重新确认安装。";
    case "INSUFFICIENT_DISK_SPACE":
      return "安装空间不足。请清理下载目录和应用所在磁盘的空间后重试。";
    case "MAC_APP_RUNNING":
    case "WINDOWS_PACKAGE_IN_USE":
      return "请退出 Codex Desktop 后重试。";
    case "JOB_ALREADY_RUNNING":
      return "另一项安装仍在进行，请等待完成后重试。";
    case "DOWNLOAD_CANCELLED":
      return "安装已取消。准备好后可重新检查并安装。";
    case "WINDOWS_DEPLOYMENT_BLOCKED":
      return "Windows 阻止了当前用户安装。请检查系统安装策略，必要时联系管理员。";
    case "WINDOWS_DEPENDENCY_MISSING":
      return "Windows 缺少安装所需的系统组件。请完成系统更新后重试。";
    case "MAC_MULTIPLE_INSTALLATIONS":
    case "MULTIPLE_INSTALLATIONS":
    case "MAC_TARGET_PATH_CONFLICT":
      return "安装位置存在冲突。请保留要使用的 Codex Desktop 安装后刷新状态。";
    default:
      return "安装未完成，请重试或打开日志目录查看详情。";
  }
}

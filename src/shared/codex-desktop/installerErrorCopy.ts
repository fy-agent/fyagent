import type { InstallerErrorDto } from "@/domain/codex-desktop";

export interface CodexInstallerErrorPresentation {
  message: string;
  title: string;
  recoverable: boolean;
  offerStagingPicker: boolean;
}

function installerErrorMessage(error: InstallerErrorDto): string {
  switch (error.details.platformErrorCode) {
    case "staging_permission_denied":
      return "所选下载临时目录无法完成文件创建、写入和清理。请检查目录权限或选择其他目录。";
    case "staging_directory_unavailable":
      return "所选下载临时目录不可用，或未通过目录身份安全检查。请重新选择现有的本地目录。";
    case "staging_space_unavailable":
      return "无法读取所选下载临时目录的可用空间。请检查磁盘连接和目录访问权限，或选择其他目录。";
    case "staging_insufficient_space":
      return "所选下载临时目录空间不足。请清理该磁盘或选择其他目录。";
    case "staging_cleanup_failed":
      return "无法安全清理下载临时目录中的探测文件。请检查目录权限或选择其他目录。";
    case "target_permission_denied":
      return "安装目录不可写。请检查目录权限后重试，或将现有 Codex Desktop 移至 ~/Applications 后刷新。";
    case "disk_space_unavailable":
      return "无法读取安装目录的可用空间。请检查磁盘是否已连接、目录是否可访问，然后重试。";
    case "helper_busy":
      return "另一项检测或安装仍在进行，请稍后重试。";
    case "shell_desktop_unavailable":
      return "无法通过 Windows 资源管理器启动安装。请确认已登录桌面且资源管理器正在运行，然后重试。";
    case "programdata_ancestor_permissions":
      return "安装目录的权限安全检查未通过，请查看日志了解详情。";
  }
  switch (error.code) {
    case "PACKAGE_IDENTITY_MISMATCH":
      return "安装安全检查未通过，请查看日志了解详情。";
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

export function codexInstallerErrorPresentation(
  error: InstallerErrorDto,
): CodexInstallerErrorPresentation {
  const platformCode = error.details.platformErrorCode;
  const stagingError =
    platformCode === "staging_permission_denied" ||
    platformCode === "staging_directory_unavailable" ||
    platformCode === "staging_space_unavailable" ||
    platformCode === "staging_insufficient_space" ||
    platformCode === "staging_cleanup_failed";
  const permissionError =
    platformCode === "target_permission_denied" ||
    platformCode === "programdata_ancestor_permissions";
  const unavailableError = platformCode === "disk_space_unavailable";
  const spaceError = error.code === "INSUFFICIENT_DISK_SPACE";
  return {
    message: installerErrorMessage(error),
    title: stagingError
      ? "下载临时目录不可用"
      : permissionError
        ? "安装权限不足或安全检查未通过"
        : spaceError
          ? "磁盘空间不足"
          : "无法检查磁盘空间",
    recoverable:
      stagingError || permissionError || unavailableError || spaceError,
    offerStagingPicker: stagingError,
  };
}

export function codexInstallerErrorCopy(error: InstallerErrorDto): string {
  return codexInstallerErrorPresentation(error).message;
}

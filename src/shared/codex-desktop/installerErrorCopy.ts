import type { InstallerErrorDto } from "@/domain/codex-desktop";

export function codexInstallerErrorCopy(error: InstallerErrorDto): string {
  if (error.details.platformErrorCode === "target_permission_denied") {
    return "安装目录不可写。请检查目录权限后重试，或将现有 Codex Desktop 移至 ~/Applications 后刷新。";
  }
  if (error.details.platformErrorCode === "disk_space_unavailable") {
    return "无法读取安装目录的可用空间。请检查磁盘是否已连接、目录是否可访问，然后重试。";
  }
  if (error.details.platformErrorCode === "helper_busy") {
    return "另一项检测或安装仍在进行，请稍后重试。";
  }
  if (error.details.platformErrorCode === "shell_desktop_unavailable") {
    return "无法通过 Windows 资源管理器启动安装。请确认已登录桌面且资源管理器正在运行，然后重试。";
  }
  if (error.details.platformErrorCode === "interactive_user_unavailable") {
    return "无法确认当前登录的 Windows 用户，请回到桌面后重试。";
  }
  if (error.details.platformErrorCode === "install_component_unconfirmed") {
    return "安装组件无法确认，请重试。";
  }
  if (error.details.platformErrorCode === "downloaded_file_moved") {
    return "刚才下载的安装包已经不在原位置，请重新检查并安装。";
  }
  switch (error.code) {
    case "PACKAGE_IDENTITY_MISMATCH":
      return "找到的安装不是 Codex 官方包，已停止。";
    case "CHECKSUM_MISMATCH":
      return "下载的安装包和官方文件不一致，请重新下载。";
    case "PACKAGE_PARSE_FAILED":
      return "安装包无法读取，请重新下载。";
    case "PACKAGE_SIGNATURE_INVALID":
      return "安装包签名没有通过，已停止安装。";
    case "DOWNLOAD_FAILED":
      return "安装包下载失败。请检查网络后重试。";
    case "METADATA_CHANGED":
      return "要安装的版本或目标已经变了，请刷新后重新确认。";
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
    case "WINDOWS_DEPLOYMENT_FAILED":
      return "安装包没有写入完成，请重试。";
    case "INSTALLATION_VERIFY_FAILED":
      return "无法确认 Codex 是否已经安装好，请刷新后再试。";
    case "LAUNCH_FAILED":
      return "Codex 没有成功打开。请手动打开一次。";
    case "MAC_MULTIPLE_INSTALLATIONS":
    case "MULTIPLE_INSTALLATIONS":
    case "MAC_TARGET_PATH_CONFLICT":
      return "安装位置存在冲突。请保留要使用的 Codex Desktop 安装后刷新状态。";
    default:
      return "安装未完成，请重试或打开日志目录查看详情。";
  }
}

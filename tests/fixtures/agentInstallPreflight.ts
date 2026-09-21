import type {
  AgentInstallPreflight,
  StartAgentActionRequest,
} from "@/shared/features/agent-install-readiness";

export function installPreflightFixture(
  request: StartAgentActionRequest,
): AgentInstallPreflight {
  return {
    contractVersion: 1,
    request,
    platform: "macos",
    architecture: "aarch64",
    versionOrChannel: "1.2.3",
    downloadUrl:
      "https://static.qoder.com.cn/qoder-work-cn/releases/latest/QoderWorkCN-arm64.dmg",
    targetLabel: "~/Applications",
    availableBytes: 10 * 1024 ** 3,
    requiredBytes: 6 * 1024 ** 3,
    artifactSizeBytes: null,
    spaceBudgetBasis: "download_limit",
    runtime: "native_installer",
    execution: "current_user",
  };
}

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
    targetLabel: "~/Applications",
    availableBytes: 10 * 1024 ** 3,
    runtime: "native_installer",
    execution: "current_user",
  };
}

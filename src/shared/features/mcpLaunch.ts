import { detectNativePlatform } from "../platform/runtime";

export type McpLaunchPlatform = ReturnType<typeof detectNativePlatform>;

export function currentMcpLaunchPlatform(): McpLaunchPlatform {
  return detectNativePlatform();
}

export function buildNpxCommand(
  packageName: string,
  extraArgs: readonly string[] = [],
  platform: McpLaunchPlatform = detectNativePlatform(),
): { command: string; args: string[] } {
  const packageArgs = ["-y", packageName, ...extraArgs];
  if (platform === "windows") {
    return { command: "cmd", args: ["/c", "npx", ...packageArgs] };
  }
  return { command: "npx", args: packageArgs };
}

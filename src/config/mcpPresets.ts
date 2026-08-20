import type { McpServer } from "../types";
import { mcpPresets, type McpPreset } from "../v2/shared/features/presets";

export { mcpPresets, type McpPreset };

export const getMcpPresetWithDescription = (
  preset: McpPreset,
  t: (key: string) => string,
): McpServer =>
  ({
    ...preset,
    description: t(`mcp.presets.${preset.id}.description`),
  }) as McpServer;

export default mcpPresets;

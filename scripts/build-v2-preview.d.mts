export interface ViteBootstrapEntries {
  scriptSource: string;
  stylesheetSources: string[];
}

export type DistributionEntry = (
  | { kind: "script" | "stylesheet"; source: string }
  | ({ kind: "vite-bootstrap" } & ViteBootstrapEntries)
) & { start: number; end: number };

export interface PreviewBuildOptions {
  distributionDirectory?: string;
  outputPath?: string;
}

export interface PreviewBuildResult {
  outputPath: string;
  scriptEntryCount: number;
  stylesheetEntryCount: number;
}

export function parseDistributionEntryAssets(
  indexHtml: string,
): DistributionEntry[];
export function parseViteBootstrapEntries(
  content: string,
): ViteBootstrapEntries | undefined;
export function resolveDistributionAsset(
  distributionDirectory: string,
  source: string,
  relativeToDirectory?: string,
): Promise<string>;
export function buildV2Preview(
  options?: PreviewBuildOptions,
): Promise<PreviewBuildResult>;
export function buildStandaloneV2Preview(
  options?: Pick<PreviewBuildOptions, "outputPath">,
): Promise<PreviewBuildResult>;

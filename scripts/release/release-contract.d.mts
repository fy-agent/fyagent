export type ReleasePlatform = "macos" | "windows";
export type ReleaseArchitecture = "universal" | "x64" | "arm64";
export type ReleaseTargetGroup =
  | "macos-universal"
  | "windows-x64"
  | "windows-arm64";
export type GitHubRunnerOS = "Windows" | "macOS";
export type GitHubRunnerArch = "X86" | "X64" | "ARM" | "ARM64";

export interface InstallerRule {
  readonly suffix: string;
  readonly platform: ReleasePlatform;
  readonly kind: "dmg" | "zip" | "exe";
  readonly architecture: ReleaseArchitecture;
}

export interface ExpectedTarget {
  readonly targetGroup: ReleaseTargetGroup;
  readonly platform: ReleasePlatform;
  readonly architecture: ReleaseArchitecture;
  readonly requestedRunnerLabel: string;
  readonly expectedRunnerOs: GitHubRunnerOS;
  readonly expectedRunnerArch: GitHubRunnerArch;
}

export interface ReleaseIdentity {
  productVersion: string;
  tag: string;
  sourceSha: string;
  repository: string;
  repositoryId: string;
  workflowPath: string;
  workflowRef: string;
  workflowSha: string;
  runId: string;
  runAttempt: string;
  event: string;
  mode: string;
  ciWorkflowPath: string;
  ciRunId: string;
  ciRunAttempt: string;
}

export interface DownloadManifestAsset {
  name: string;
  platform: ReleasePlatform;
  architecture: ReleaseArchitecture;
  format: InstallerRule["kind"];
  sizeBytes: number;
  sha256: string;
  url: string;
}

export interface DownloadManifest {
  schema: "fyagent-download-manifest/v3";
  product: "FyAgent";
  version: string;
  tag: string;
  sourceSha: string;
  publishedAt: string;
  assets: DownloadManifestAsset[];
}

export interface PlatformBuildTargetMetadata {
  schema: "fyagent-platform-build/v2";
  targetGroup: ReleaseTargetGroup;
  platform: ReleasePlatform;
  architecture: ReleaseArchitecture;
  runner: {
    requestedLabel: string;
    context: {
      os: GitHubRunnerOS;
      arch: GitHubRunnerArch;
    };
  };
  toolchain: {
    node: string;
    pnpm: string;
    rustc: string;
  };
}

export interface PlatformBuildMetadataRecord
  extends PlatformBuildTargetMetadata {
  identity: ReleaseIdentity;
}

export interface BuildMetadata {
  schema: "fyagent-build-metadata/v2";
  product: "FyAgent";
  version: string;
  tag: string;
  sourceSha: string;
  repository: {
    nameWithOwner: string;
    id: string;
  };
  workflow: {
    path: string;
    ref: string;
    sha: string;
    runId: string;
    runAttempt: string;
    event: string;
    mode: string;
  };
  requiredCi: {
    path: string;
    runId: string;
    runAttempt: string;
    job: "CI / Required";
    conclusion: "success";
  };
  generatedAt: string;
  targets: PlatformBuildTargetMetadata[];
}

export const PRODUCT_NAME: "FyAgent";
export const EXPECTED_REPOSITORY: "fy-agent/fyagent";
export const EXPECTED_REPOSITORY_ID: "1313497021";
export const PREFLIGHT_BRANCH: "dev/laiyongjie";
export const RELEASE_BRANCH: "main";
export const RELEASE_WORKFLOW_PATH: ".github/workflows/release.yml";
export const CI_WORKFLOW_PATH: ".github/workflows/ci.yml";
export const DOWNLOAD_MANIFEST_NAME: "download-manifest.json";
export const BUILD_METADATA_NAME: "build-metadata.json";
export const WINDOWS_SIGNING_STATUS_NAME: "signing-status.json";
export const ATTESTATION_BUNDLE_NAME: "artifact-attestation.sigstore.json";

export const GITHUB_RUNNER_ARCHITECTURES: readonly GitHubRunnerArch[];
export const INSTALLER_RULES: readonly InstallerRule[];
export const EXPECTED_TARGETS: readonly ExpectedTarget[];
export const EXPECTED_INSTALLERS_BY_TARGET: Readonly<
  Record<ReleaseTargetGroup, readonly number[]>
>;
export const WINDOWS_SIGNING_FRAGMENTS_BY_TARGET: Readonly<
  Record<"windows-x64" | "windows-arm64", string>
>;

export function assertWindowsBundleVersion(version: string): void;
export function assertReleaseIdentity(identity: {
  version: string;
  tag: string;
  sourceSha: string;
}): void;
export function expectedInstallerNames(version: string): string[];
export function expectedAttestationSubjectNames(version: string): string[];
export function expectedReleaseAttachmentNames(version: string): string[];
export function assertExactFileSet(
  directory: string,
  expectedNames: readonly string[],
  label: string,
): string[];
export function assertExactDirectorySet(
  directory: string,
  expectedNames: readonly string[],
  label: string,
): void;
export function assertExactInstallerSet(
  directory: string,
  version: string,
): string[];
export function sha256File(filePath: string): Promise<string>;
export function buildDownloadManifest(input: {
  assetsDirectory: string;
  version: string;
  tag: string;
  sourceSha: string;
  baseUrl: string;
  publishedAt: string;
}): Promise<DownloadManifest>;
export function buildBuildMetadata(input: {
  metadataDirectory: string;
  identity: ReleaseIdentity;
  generatedAt: string;
}): BuildMetadata;

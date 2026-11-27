import { useQuery, useQueryClient } from "@tanstack/react-query";
import { useId, useState } from "react";

import { useFeatures } from "../provider";
import { featureKeys } from "../queries";
import type {
  RemotePathMapping,
  RemotePushApplyResult,
  RemotePushPreviewResult,
  SessionProjectionMode,
  SshConnectionTestResult,
  SshHostEntry,
  SshPushProfile,
  WorkspacePackExportOptions,
  WorkspacePackExportOutcome,
  WorkspacePackExportPreview,
  WorkspacePackImportOutcome,
  WorkspacePackImportPreview,
} from "../workspace-transfer";
import { Button } from "../../ui/Button";
import { Dialog } from "../../ui/Dialog";
import type { DialogOriginRef } from "../../ui/dialogOrigin";
import { FeatureTabPanel, FeatureTabs } from "../../ui/FeatureTabs";
import {
  Badge,
  Checkbox,
  InlineNotice,
  Input,
  Spinner,
} from "../../ui/primitives";
import "./workspace-transfer.css";

type TransferTab = "export" | "import" | "ssh" | "wsl";

const TRANSFER_TABS: ReadonlyArray<{ id: TransferTab; label: string }> = [
  { id: "export", label: "打包迁移包 (≤100MB)" },
  { id: "import", label: "导入迁移包" },
  { id: "ssh", label: "推送到 SSH 远端" },
  { id: "wsl", label: "同步到 WSL" },
];

interface PathMappingFormState {
  homeRoot: string;
  claudeDir: string;
  codexDir: string;
  opencodeDir: string;
  ompDir: string;
}

function emptyPathMappingForm(): PathMappingFormState {
  return {
    homeRoot: "",
    claudeDir: "",
    codexDir: "",
    opencodeDir: "",
    ompDir: "",
  };
}

function toOptionalPathMapping(
  form: PathMappingFormState,
): RemotePathMapping | null {
  const mapping: RemotePathMapping = {};
  if (form.homeRoot.trim()) mapping.homeRoot = form.homeRoot.trim();
  if (form.claudeDir.trim()) mapping.claudeDir = form.claudeDir.trim();
  if (form.codexDir.trim()) mapping.codexDir = form.codexDir.trim();
  if (form.opencodeDir.trim()) mapping.opencodeDir = form.opencodeDir.trim();
  if (form.ompDir.trim()) mapping.ompDir = form.ompDir.trim();
  return Object.keys(mapping).length > 0 ? mapping : null;
}

function PathMappingFields({
  value,
  onChange,
  idPrefix,
}: {
  value: PathMappingFormState;
  onChange: (next: PathMappingFormState) => void;
  idPrefix: string;
}) {
  return (
    <details className="fy-transfer-mapping-details">
      <summary>自定义目录映射（选填，留空默认使用 $HOME 标准目录）</summary>
      <div className="fy-transfer-grid fy-transfer-mapping-body">
        <label className="fy-transfer-field">
          <span>目标根目录</span>
          <Input
            id={`${idPrefix}-home-root`}
            value={value.homeRoot}
            onChange={(e) => onChange({ ...value, homeRoot: e.target.value })}
            placeholder="留空默认 ~，如 ~/work-env"
          />
        </label>
        <label className="fy-transfer-field">
          <span>Claude 目录</span>
          <Input
            id={`${idPrefix}-claude-dir`}
            value={value.claudeDir}
            onChange={(e) => onChange({ ...value, claudeDir: e.target.value })}
            placeholder=".claude"
          />
        </label>
        <label className="fy-transfer-field">
          <span>Codex 目录</span>
          <Input
            id={`${idPrefix}-codex-dir`}
            value={value.codexDir}
            onChange={(e) => onChange({ ...value, codexDir: e.target.value })}
            placeholder=".codex"
          />
        </label>
        <label className="fy-transfer-field">
          <span>OpenCode 目录</span>
          <Input
            id={`${idPrefix}-opencode-dir`}
            value={value.opencodeDir}
            onChange={(e) =>
              onChange({ ...value, opencodeDir: e.target.value })
            }
            placeholder=".config/opencode"
          />
        </label>
        <label className="fy-transfer-field">
          <span>omp 目录</span>
          <Input
            id={`${idPrefix}-omp-dir`}
            value={value.ompDir}
            onChange={(e) => onChange({ ...value, ompDir: e.target.value })}
            placeholder=".omp/agent"
          />
        </label>
      </div>
    </details>
  );
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KiB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} MiB`;
}

function defaultExportOptions(): WorkspacePackExportOptions {
  return {
    includeModels: true,
    includeCustomTemplates: true,
    includeMcp: true,
    includePrompts: true,
    includeSkills: true,
    includeSessions: true,
    includeApiKeys: false,
    sessionMode: "full_dialogue",
    sessionMaxAgeDays: 90,
    sessionMaxCount: 200,
  };
}

export function WorkspaceTransferDialog({
  open,
  originRef,
  onOpenChange,
}: {
  open: boolean;
  originRef?: DialogOriginRef;
  onOpenChange: (open: boolean) => void;
}) {
  const { ports } = useFeatures();
  const queryClient = useQueryClient();
  const tabsId = useId();
  const [tab, setTab] = useState<TransferTab>("export");

  // ── Export state ──
  const [exportOptions, setExportOptions] =
    useState<WorkspacePackExportOptions>(defaultExportOptions);
  const [exportPassphrase, setExportPassphrase] = useState("");
  const [exportPreview, setExportPreview] =
    useState<WorkspacePackExportPreview | null>(null);
  const [exportOutcome, setExportOutcome] =
    useState<WorkspacePackExportOutcome | null>(null);
  const [exportBusy, setExportBusy] = useState(false);
  const [exportError, setExportError] = useState<string | null>(null);

  // ── Import state ──
  const [importPath, setImportPath] = useState<string | null>(null);
  const [importPassphrase, setImportPassphrase] = useState("");
  const [importPreview, setImportPreview] =
    useState<WorkspacePackImportPreview | null>(null);
  const [importFlags, setImportFlags] = useState({
    importModels: true,
    importCustomTemplates: true,
    importMcp: true,
    importPrompts: true,
    importSkills: true,
    importSessions: true,
  });
  const [importOutcome, setImportOutcome] =
    useState<WorkspacePackImportOutcome | null>(null);
  const [importBusy, setImportBusy] = useState(false);
  const [importError, setImportError] = useState<string | null>(null);

  // ── SSH push state ──
  const sshHostsQuery = useQuery({
    queryKey: featureKeys.sshHosts,
    queryFn: () => ports.workspaceTransfer.listSshHosts(),
    enabled: open && tab === "ssh",
  });
  const sshProfilesQuery = useQuery({
    queryKey: featureKeys.sshPushProfiles,
    queryFn: () => ports.workspaceTransfer.listSshPushProfiles(),
    enabled: open && tab === "ssh",
  });
  const [sshHost, setSshHost] = useState("");
  const [sshUser, setSshUser] = useState("");
  const [sshPort, setSshPort] = useState("");
  const [sshIdentityFile, setSshIdentityFile] = useState("");
  const [sshPathMapping, setSshPathMapping] =
    useState<PathMappingFormState>(emptyPathMappingForm);
  const [activeProfileId, setActiveProfileId] = useState<string | null>(null);
  const [saveProfileOpen, setSaveProfileOpen] = useState(false);
  const [profileName, setProfileName] = useState("");
  const [profileSaving, setProfileSaving] = useState(false);
  const [sshSelection, setSshSelection] = useState({
    includeModels: true,
    includeMcp: true,
    includePrompts: true,
    includeSkills: true,
  });
  const [sshTestResult, setSshTestResult] =
    useState<SshConnectionTestResult | null>(null);
  const [sshPreview, setSshPreview] = useState<RemotePushPreviewResult | null>(
    null,
  );
  const [sshOutcome, setSshOutcome] = useState<RemotePushApplyResult | null>(
    null,
  );
  const [sshBusy, setSshBusy] = useState(false);
  const [sshError, setSshError] = useState<string | null>(null);

  // ── WSL sync state ──
  const wslDistrosQuery = useQuery({
    queryKey: featureKeys.wslDistros,
    queryFn: () => ports.workspaceTransfer.listWslDistros(),
    enabled: open && tab === "wsl",
  });
  const [wslDistroInput, setWslDistroInput] = useState("");
  const [wslPathMapping, setWslPathMapping] =
    useState<PathMappingFormState>(emptyPathMappingForm);
  const [wslSelection, setWslSelection] = useState({
    includeModels: true,
    includeMcp: true,
    includePrompts: true,
    includeSkills: true,
  });
  const [wslPreview, setWslPreview] = useState<RemotePushPreviewResult | null>(
    null,
  );
  const [wslOutcome, setWslOutcome] = useState<RemotePushApplyResult | null>(
    null,
  );
  const [wslBusy, setWslBusy] = useState(false);
  const [wslError, setWslError] = useState<string | null>(null);

  const wslStatus = wslDistrosQuery.data;
  const effectiveWslDistro =
    wslDistroInput.trim() ||
    wslStatus?.distros.find((d) => d.isDefault)?.name ||
    wslStatus?.distros[0]?.name ||
    "";

  const updateExportOption = <K extends keyof WorkspacePackExportOptions>(
    key: K,
    value: WorkspacePackExportOptions[K],
  ) => {
    setExportOptions((prev) => ({ ...prev, [key]: value }));
    setExportPreview(null);
    setExportOutcome(null);
    setExportError(null);
  };

  const handlePreviewExport = async () => {
    setExportBusy(true);
    setExportError(null);
    setExportOutcome(null);
    try {
      const result = await ports.workspaceTransfer.previewExport(exportOptions);
      setExportPreview(result);
    } catch (err) {
      setExportError(
        err instanceof Error ? err.message : "估算迁移包体积失败，请重试。",
      );
    } finally {
      setExportBusy(false);
    }
  };

  const handleRunExport = async () => {
    if (exportOptions.includeApiKeys && exportPassphrase.trim().length < 6) {
      setExportError("包含 API Key 导出时请设置至少 6 位加密口令。");
      return;
    }
    setExportBusy(true);
    setExportError(null);
    try {
      const today = new Date().toISOString().slice(0, 10);
      const targetPath = await ports.workspaceTransfer.pickExportPath(
        `fyagent-${today}.fyagent-pack`,
      );
      if (!targetPath) return;
      const outcome = await ports.workspaceTransfer.exportPack({
        targetPath,
        options: exportOptions,
        passphrase: exportOptions.includeApiKeys
          ? exportPassphrase.trim()
          : null,
      });
      setExportOutcome(outcome);
      setExportPassphrase("");
    } catch (err) {
      setExportError(
        err instanceof Error ? err.message : "导出迁移包失败，请检查保存路径。",
      );
    } finally {
      setExportBusy(false);
    }
  };

  const inspectFile = async (path: string, passphrase?: string) => {
    setImportBusy(true);
    setImportError(null);
    setImportOutcome(null);
    try {
      const preview = await ports.workspaceTransfer.inspectPack({
        path,
        passphrase: passphrase?.trim() || null,
      });
      setImportPath(path);
      setImportPreview(preview);
    } catch (err) {
      setImportError(
        err instanceof Error ? err.message : "无法读取或校验该迁移包。",
      );
    } finally {
      setImportBusy(false);
    }
  };

  const handlePickImportFile = async () => {
    setImportError(null);
    try {
      const path = await ports.workspaceTransfer.pickImportFile();
      if (!path) return;
      setImportPassphrase("");
      await inspectFile(path);
    } catch (err) {
      setImportError(
        err instanceof Error ? err.message : "打开迁移包文件失败。",
      );
    }
  };

  const handleApplyImport = async () => {
    if (!importPath) return;
    setImportBusy(true);
    setImportError(null);
    try {
      const outcome = await ports.workspaceTransfer.applyImport({
        path: importPath,
        passphrase: importPassphrase.trim() || null,
        ...importFlags,
      });
      setImportOutcome(outcome);
      setImportPassphrase("");
      await Promise.all([
        queryClient.invalidateQueries({
          queryKey: featureKeys.modelHealthOverview,
        }),
        queryClient.invalidateQueries({
          queryKey: featureKeys.customGatewayTemplates,
        }),
        queryClient.invalidateQueries({
          queryKey: featureKeys.providerSummary("codex"),
        }),
        queryClient.invalidateQueries({
          queryKey: featureKeys.providerSummary("claude"),
        }),
        queryClient.invalidateQueries({
          queryKey: featureKeys.providerSummary("grokbuild"),
        }),
        queryClient.invalidateQueries({
          queryKey: featureKeys.workbuddyStatus,
        }),
        queryClient.invalidateQueries({
          queryKey: featureKeys.workbuddyModelIds,
        }),
        queryClient.invalidateQueries({
          queryKey: featureKeys.openCodeModelSnapshot,
        }),
        queryClient.invalidateQueries({ queryKey: featureKeys.ompConfig }),
        queryClient.invalidateQueries({
          queryKey: featureKeys.codexReloadStatus,
        }),
        queryClient.invalidateQueries({ queryKey: featureKeys.mcp }),
        queryClient.invalidateQueries({ queryKey: featureKeys.skills }),
        queryClient.invalidateQueries({
          queryKey: featureKeys.prompts("claude"),
        }),
        queryClient.invalidateQueries({
          queryKey: featureKeys.prompts("codex"),
        }),
        queryClient.invalidateQueries({
          queryKey: featureKeys.prompts("gemini"),
        }),
        queryClient.invalidateQueries({
          queryKey: featureKeys.prompts("opencode"),
        }),
        queryClient.invalidateQueries({
          queryKey: featureKeys.promptLiveFile("claude"),
        }),
        queryClient.invalidateQueries({
          queryKey: featureKeys.promptLiveFile("codex"),
        }),
        queryClient.invalidateQueries({
          queryKey: featureKeys.promptLiveFile("gemini"),
        }),
        queryClient.invalidateQueries({
          queryKey: featureKeys.promptLiveFile("opencode"),
        }),
      ]);
    } catch (err) {
      setImportError(
        err instanceof Error ? err.message : "导入迁移包失败，原配置未改动。",
      );
    } finally {
      setImportBusy(false);
    }
  };

  const buildValidatedSshTarget = () => {
    const trimmedPort = sshPort.trim();
    let port: number | null = null;
    if (trimmedPort) {
      if (!/^\d+$/.test(trimmedPort)) {
        return {
          ok: false as const,
          error: "SSH 端口必须为 1–65535 之间的正整数。",
        };
      }
      const parsedPort = Number(trimmedPort);
      if (
        !Number.isInteger(parsedPort) ||
        parsedPort < 1 ||
        parsedPort > 65535
      ) {
        return {
          ok: false as const,
          error: "SSH 端口必须为 1–65535 之间的正整数。",
        };
      }
      port = parsedPort;
    }
    return {
      ok: true as const,
      target: {
        host: sshHost.trim(),
        user: sshUser.trim() || null,
        port,
        identityFile: sshIdentityFile.trim() || null,
      },
    };
  };

  const resetSshTransientState = () => {
    setSshTestResult(null);
    setSshPreview(null);
    setSshOutcome(null);
    setSshError(null);
  };

  const applyHostEntry = (entry: SshHostEntry) => {
    setActiveProfileId(null);
    setSshHost(entry.alias);
    setSshUser(entry.user ?? "");
    setSshPort(entry.port ? String(entry.port) : "");
    setSshIdentityFile(entry.identityFile ?? "");
    setSshPathMapping(emptyPathMappingForm());
    resetSshTransientState();
  };

  const applySavedProfile = (profile: SshPushProfile) => {
    setActiveProfileId(profile.id);
    setProfileName(profile.name);
    setSshHost(profile.host);
    setSshUser(profile.user ?? "");
    setSshPort(profile.port ? String(profile.port) : "");
    setSshIdentityFile(profile.identityFile ?? "");
    setSshPathMapping({
      homeRoot: profile.pathMapping.homeRoot ?? "",
      claudeDir: profile.pathMapping.claudeDir ?? "",
      codexDir: profile.pathMapping.codexDir ?? "",
      opencodeDir: profile.pathMapping.opencodeDir ?? "",
      ompDir: profile.pathMapping.ompDir ?? "",
    });
    resetSshTransientState();
  };

  const handleSaveProfile = async () => {
    if (!sshHost.trim()) {
      setSshError("请先填写 SSH 主机名再保存常用主机。");
      return;
    }
    const validated = buildValidatedSshTarget();
    if (!validated.ok) {
      setSshError(validated.error);
      return;
    }
    const trimmedName = profileName.trim() || sshHost.trim();
    setProfileSaving(true);
    setSshError(null);
    try {
      const saved = await ports.workspaceTransfer.saveSshPushProfile({
        ...(activeProfileId ? { id: activeProfileId } : {}),
        name: trimmedName,
        host: validated.target.host,
        user: validated.target.user,
        port: validated.target.port,
        identityFile: validated.target.identityFile,
        pathMapping: toOptionalPathMapping(sshPathMapping),
      });
      setActiveProfileId(saved.id);
      setSaveProfileOpen(false);
      await queryClient.invalidateQueries({
        queryKey: featureKeys.sshPushProfiles,
      });
    } catch (err) {
      setSshError(
        err instanceof Error ? err.message : "保存常用 SSH 主机失败。",
      );
    } finally {
      setProfileSaving(false);
    }
  };

  const handleDeleteProfile = async (profile: SshPushProfile) => {
    setProfileSaving(true);
    setSshError(null);
    try {
      await ports.workspaceTransfer.deleteSshPushProfile(profile.id);
      if (activeProfileId === profile.id) {
        setActiveProfileId(null);
      }
      await queryClient.invalidateQueries({
        queryKey: featureKeys.sshPushProfiles,
      });
    } catch (err) {
      setSshError(
        err instanceof Error ? err.message : "删除常用 SSH 主机失败。",
      );
    } finally {
      setProfileSaving(false);
    }
  };

  const handleTestSsh = async () => {
    if (!sshHost.trim()) {
      setSshError("请先选择或填写 SSH 主机名。");
      return;
    }
    const validated = buildValidatedSshTarget();
    if (!validated.ok) {
      setSshError(validated.error);
      return;
    }
    setSshBusy(true);
    setSshError(null);
    try {
      const result = await ports.workspaceTransfer.testSshTarget(
        validated.target,
      );
      setSshTestResult(result);
    } catch (err) {
      setSshError(err instanceof Error ? err.message : "测试 SSH 连接失败。");
    } finally {
      setSshBusy(false);
    }
  };

  const handlePreviewSsh = async () => {
    if (!sshHost.trim()) {
      setSshError("请先选择或填写 SSH 主机名。");
      return;
    }
    const validated = buildValidatedSshTarget();
    if (!validated.ok) {
      setSshError(validated.error);
      return;
    }
    setSshBusy(true);
    setSshError(null);
    setSshOutcome(null);
    try {
      const pathMapping = toOptionalPathMapping(sshPathMapping);
      const preview = await ports.workspaceTransfer.previewSshPush({
        target: validated.target,
        selection: sshSelection,
        ...(pathMapping ? { pathMapping } : {}),
      });
      setSshPreview(preview);
    } catch (err) {
      setSshError(
        err instanceof Error ? err.message : "生成远端推送清单失败。",
      );
    } finally {
      setSshBusy(false);
    }
  };

  const handleApplySsh = async () => {
    if (!sshHost.trim()) return;
    const validated = buildValidatedSshTarget();
    if (!validated.ok) {
      setSshError(validated.error);
      return;
    }
    setSshBusy(true);
    setSshError(null);
    try {
      const pathMapping = toOptionalPathMapping(sshPathMapping);
      const outcome = await ports.workspaceTransfer.applySshPush({
        target: validated.target,
        selection: sshSelection,
        ...(pathMapping ? { pathMapping } : {}),
      });
      setSshOutcome(outcome);
    } catch (err) {
      setSshError(err instanceof Error ? err.message : "远程配置推送失败。");
    } finally {
      setSshBusy(false);
    }
  };

  const resetWslTransientState = () => {
    setWslPreview(null);
    setWslOutcome(null);
    setWslError(null);
  };

  const handlePreviewWsl = async () => {
    if (!effectiveWslDistro) {
      setWslError("请先选择或填写目标 WSL 发行版名称。");
      return;
    }
    setWslBusy(true);
    setWslError(null);
    setWslOutcome(null);
    try {
      const pathMapping = toOptionalPathMapping(wslPathMapping);
      const preview = await ports.workspaceTransfer.previewWslSync({
        distro: effectiveWslDistro,
        selection: wslSelection,
        ...(pathMapping ? { pathMapping } : {}),
      });
      setWslPreview(preview);
    } catch (err) {
      setWslError(
        err instanceof Error ? err.message : "生成 WSL 同步清单失败。",
      );
    } finally {
      setWslBusy(false);
    }
  };

  const handleApplyWsl = async () => {
    if (!effectiveWslDistro) return;
    setWslBusy(true);
    setWslError(null);
    try {
      const pathMapping = toOptionalPathMapping(wslPathMapping);
      const outcome = await ports.workspaceTransfer.applyWslSync({
        distro: effectiveWslDistro,
        selection: wslSelection,
        ...(pathMapping ? { pathMapping } : {}),
      });
      setWslOutcome(outcome);
    } catch (err) {
      setWslError(err instanceof Error ? err.message : "同步到 WSL 失败。");
    } finally {
      setWslBusy(false);
    }
  };

  const handleClose = () => {
    setExportPassphrase("");
    setImportPassphrase("");
    onOpenChange(false);
  };

  const budgetPercent = exportPreview
    ? Math.min(
        100,
        Math.round(
          (exportPreview.breakdown.estimatedCompressedBytes /
            exportPreview.breakdown.hardLimitBytes) *
            100,
        ),
      )
    : 0;
  const budgetWarning =
    exportPreview !== null &&
    exportPreview.breakdown.estimatedCompressedBytes >=
      exportPreview.breakdown.softWarningBytes;

  return (
    <Dialog
      open={open}
      size="wide"
      originRef={originRef}
      onOpenChange={(next) => {
        if (!next) {
          setExportPassphrase("");
          setImportPassphrase("");
        }
        onOpenChange(next);
      }}
      title="环境迁移与远程推送"
      description="打包 ≤100 MiB 投影式迁移包带入新电脑，或免客户端直接同步配置至 SSH 远端主机与 Windows WSL 发行版。"
      actions={
        <>
          <Button onClick={handleClose}>关闭</Button>
          {tab === "export" ? (
            <>
              <Button
                disabled={exportBusy}
                onClick={() => void handlePreviewExport()}
              >
                {exportBusy ? "正在估算…" : "估算体积预算"}
              </Button>
              <Button
                className="fy-control-button-primary"
                disabled={exportBusy}
                onClick={() => void handleRunExport()}
              >
                {exportBusy ? "正在打包…" : "生成 .fyagent-pack 迁移包"}
              </Button>
            </>
          ) : tab === "import" ? (
            <Button
              className="fy-control-button-primary"
              disabled={importBusy || !importPreview}
              onClick={() => void handleApplyImport()}
            >
              {importBusy ? "正在导入…" : "确认导入所选内容"}
            </Button>
          ) : tab === "ssh" ? (
            sshPreview && !sshOutcome ? (
              <>
                <Button
                  disabled={sshBusy}
                  onClick={() => {
                    setSshPreview(null);
                    setSshOutcome(null);
                  }}
                >
                  返回修改
                </Button>
                <Button
                  className="fy-control-button-primary"
                  disabled={sshBusy || sshPreview.files.length === 0}
                  onClick={() => void handleApplySsh()}
                >
                  {sshBusy ? "正在推送…" : "确认推送配置文件"}
                </Button>
              </>
            ) : (
              <Button
                className="fy-control-button-primary"
                disabled={sshBusy}
                onClick={() => void handlePreviewSsh()}
              >
                {sshBusy ? "正在探测…" : "预览远端推送清单"}
              </Button>
            )
          ) : wslPreview && !wslOutcome ? (
            <>
              <Button
                disabled={wslBusy}
                onClick={() => {
                  setWslPreview(null);
                  setWslOutcome(null);
                }}
              >
                返回修改
              </Button>
              <Button
                className="fy-control-button-primary"
                disabled={
                  wslBusy ||
                  wslPreview.files.length === 0 ||
                  wslStatus?.supported === false
                }
                onClick={() => void handleApplyWsl()}
              >
                {wslBusy ? "正在同步…" : "确认同步到 WSL"}
              </Button>
            </>
          ) : (
            <Button
              className="fy-control-button-primary"
              disabled={wslBusy || wslStatus?.supported === false}
              onClick={() => void handlePreviewWsl()}
            >
              {wslBusy ? "正在探测…" : "预览 WSL 同步清单"}
            </Button>
          )}
        </>
      }
    >
      <div className="fy-transfer-body">
        <FeatureTabs
          id={tabsId}
          label="迁移与推送模式"
          options={TRANSFER_TABS}
          value={tab}
          onChange={setTab}
        />

        <FeatureTabPanel
          tabsId={tabsId}
          value="export"
          active={tab === "export"}
          layout="flow"
        >
          <div className="fy-transfer-body">
            <div className="fy-transfer-card">
              <strong>1. 选择要打包的配置与投影内容</strong>
              <div className="fy-transfer-checks">
                <label className="fy-transfer-check-row">
                  <Checkbox
                    checked={exportOptions.includeModels}
                    onCheckedChange={(v) =>
                      updateExportOption("includeModels", Boolean(v))
                    }
                    label="各端模型配置"
                  />
                  <span>各端模型配置</span>
                </label>
                <label className="fy-transfer-check-row">
                  <Checkbox
                    checked={exportOptions.includeCustomTemplates}
                    onCheckedChange={(v) =>
                      updateExportOption("includeCustomTemplates", Boolean(v))
                    }
                    label="我的自定义模板"
                  />
                  <span>我的自定义模板</span>
                </label>
                <label className="fy-transfer-check-row">
                  <Checkbox
                    checked={exportOptions.includeMcp}
                    onCheckedChange={(v) =>
                      updateExportOption("includeMcp", Boolean(v))
                    }
                    label="MCP 服务器清单"
                  />
                  <span>MCP 服务器清单</span>
                </label>
                <label className="fy-transfer-check-row">
                  <Checkbox
                    checked={exportOptions.includePrompts}
                    onCheckedChange={(v) =>
                      updateExportOption("includePrompts", Boolean(v))
                    }
                    label="全局提示词规则"
                  />
                  <span>全局提示词规则</span>
                </label>
                <label className="fy-transfer-check-row">
                  <Checkbox
                    checked={exportOptions.includeSkills}
                    onCheckedChange={(v) =>
                      updateExportOption("includeSkills", Boolean(v))
                    }
                    label="Skills 纯文本资产"
                  />
                  <span>Skills 纯文本资产</span>
                </label>
                <label className="fy-transfer-check-row">
                  <Checkbox
                    checked={exportOptions.includeSessions}
                    onCheckedChange={(v) =>
                      updateExportOption("includeSessions", Boolean(v))
                    }
                    label="历史会话轻量投影"
                  />
                  <span>历史会话轻量投影</span>
                </label>
              </div>
            </div>

            {exportOptions.includeSessions ? (
              <div className="fy-transfer-card">
                <strong>
                  2. 会话轻量投影策略（剔除工具日志与附件，严格 ≤ 100 MiB）
                </strong>
                <div className="fy-transfer-chip-row">
                  {(
                    [
                      ["full_dialogue", "完整对话投影（保留全部问答纯文本）"],
                      [
                        "compact_context",
                        "硬件精简投影（开头目标 + 末尾 6 轮）",
                      ],
                    ] as ReadonlyArray<[SessionProjectionMode, string]>
                  ).map(([mode, label]) => (
                    <Button
                      key={mode}
                      className={
                        exportOptions.sessionMode === mode
                          ? "fy-control-button-primary"
                          : undefined
                      }
                      onClick={() => updateExportOption("sessionMode", mode)}
                    >
                      {label}
                    </Button>
                  ))}
                </div>
                <div className="fy-transfer-chip-row">
                  <span>时间范围：</span>
                  {(
                    [
                      [30, "最近 30 天"],
                      [90, "最近 90 天"],
                      [null, "全部时间"],
                    ] as ReadonlyArray<[number | null, string]>
                  ).map(([days, label]) => (
                    <Button
                      key={String(days)}
                      className={
                        (exportOptions.sessionMaxAgeDays ?? null) === days
                          ? "fy-control-button-primary"
                          : undefined
                      }
                      onClick={() =>
                        updateExportOption("sessionMaxAgeDays", days)
                      }
                    >
                      {label}
                    </Button>
                  ))}
                </div>
              </div>
            ) : null}

            <div className="fy-transfer-card">
              <label className="fy-transfer-check-row">
                <Checkbox
                  checked={exportOptions.includeApiKeys}
                  onCheckedChange={(v) =>
                    updateExportOption("includeApiKeys", Boolean(v))
                  }
                  label="包含 API Key（AES-256-GCM 个人口令加密）"
                />
                <strong>
                  包含 API Key（个人口令加密，OAuth 账号永不导出）
                </strong>
              </label>
              {exportOptions.includeApiKeys ? (
                <label className="fy-transfer-field">
                  <span>设置加密口令（至少 6 位，导入新电脑时需输入）</span>
                  <Input
                    type="password"
                    value={exportPassphrase}
                    onChange={(e) => {
                      setExportPassphrase(e.target.value);
                      setExportError(null);
                    }}
                    placeholder="输入不少于 6 位的迁移口令"
                    autoComplete="new-password"
                  />
                </label>
              ) : null}
            </div>

            {exportBusy ? (
              <div className="fy-transfer-loading">
                <Spinner label="正在扫描并估算迁移包体积" />
                <span>正在计算轻量投影与压缩体积…</span>
              </div>
            ) : null}
            {exportError ? (
              <InlineNotice tone="error">{exportError}</InlineNotice>
            ) : null}

            {exportPreview ? (
              <section
                className="fy-transfer-budget"
                aria-label="100 MiB 体积预算仪"
              >
                <div className="fy-transfer-budget-header">
                  <strong>
                    预估压缩体积：
                    {formatBytes(
                      exportPreview.breakdown.estimatedCompressedBytes,
                    )}{" "}
                    / 100.00 MiB 硬上限
                  </strong>
                  <Badge tone={budgetWarning ? "warning" : "accent"}>
                    {budgetWarning ? "接近上限" : "硬件载体友好"}
                  </Badge>
                </div>
                <div className="fy-transfer-meter" aria-hidden="true">
                  <div
                    className="fy-transfer-meter-fill"
                    data-tone={budgetWarning ? "warning" : undefined}
                    style={{ width: `${Math.max(budgetPercent, 2)}%` }}
                  />
                </div>
                <div className="fy-transfer-stats">
                  <span>
                    模型配置：{exportPreview.counts.models} 端（
                    {formatBytes(exportPreview.breakdown.modelsBytes)}）
                  </span>
                  <span>
                    自定义模板：{exportPreview.counts.customTemplates} 个
                  </span>
                  <span>
                    MCP：{exportPreview.counts.mcpServers} 项（
                    {formatBytes(exportPreview.breakdown.mcpBytes)}）
                  </span>
                  <span>
                    提示词：{exportPreview.counts.prompts} 条（
                    {formatBytes(exportPreview.breakdown.promptsBytes)}）
                  </span>
                  <span>
                    Skills：{exportPreview.counts.skills} 个（
                    {formatBytes(exportPreview.breakdown.skillsBytes)}）
                  </span>
                  <span>
                    会话投影：{exportPreview.counts.sessionsIncluded} 条（原始{" "}
                    {formatBytes(exportPreview.breakdown.sessionsBytes)}）
                  </span>
                </div>
                {exportPreview.warnings.map((w, idx) => (
                  <InlineNotice key={idx} tone="warning">
                    {w}
                  </InlineNotice>
                ))}
              </section>
            ) : null}

            {exportOutcome ? (
              <>
                <InlineNotice tone="info">
                  已生成迁移包 <code>{exportOutcome.path}</code>（压缩后{" "}
                  <strong>{formatBytes(exportOutcome.compressedBytes)}</strong>
                  ，SHA-256{" "}
                  <code>{exportOutcome.fileSha256.slice(0, 16)}…</code>
                  ，含 {exportOutcome.counts.models} 端模型、
                  {exportOutcome.counts.skills} 个 Skills、
                  {exportOutcome.counts.sessionsIncluded} 条会话投影）。
                </InlineNotice>
                {exportOutcome.warnings.map((w, idx) => (
                  <InlineNotice key={`exp-w-${idx}`} tone="warning">
                    {w}
                  </InlineNotice>
                ))}
              </>
            ) : null}
          </div>
        </FeatureTabPanel>

        <FeatureTabPanel
          tabsId={tabsId}
          value="import"
          active={tab === "import"}
          layout="flow"
        >
          <div className="fy-transfer-body">
            <div className="fy-transfer-card">
              <div className="fy-transfer-field-row">
                <Button
                  disabled={importBusy}
                  onClick={() => void handlePickImportFile()}
                >
                  选择 .fyagent-pack 迁移包
                </Button>
                {importPath ? <code>{importPath}</code> : null}
              </div>

              {importPreview ? (
                <>
                  <div className="fy-transfer-stats">
                    <span>来源版本：{importPreview.manifest.appVersion}</span>
                    <span>来源平台：{importPreview.manifest.platform}</span>
                    <span>
                      包体积：
                      {formatBytes(importPreview.manifest.compressedBytes)}
                    </span>
                    <span>模型：{importPreview.manifest.counts.models} 端</span>
                    <span>
                      自定义模板：
                      {importPreview.manifest.counts.customTemplates} 个
                    </span>
                    <span>
                      MCP：{importPreview.manifest.counts.mcpServers} 项
                    </span>
                    <span>
                      Skills：{importPreview.manifest.counts.skills} 个
                    </span>
                    <span>
                      会话：{importPreview.manifest.counts.sessionsIncluded}{" "}
                      条（原生可还原 {importPreview.strictSessionCount} 条）
                    </span>
                  </div>

                  {importPreview.secretsLocked ? (
                    <div className="fy-transfer-field-row">
                      <label className="fy-transfer-field">
                        <span>该迁移包包含加密 API Key，请输入口令解锁</span>
                        <Input
                          type="password"
                          value={importPassphrase}
                          onChange={(e) => {
                            setImportPassphrase(e.target.value);
                            setImportError(null);
                          }}
                          placeholder="输入导出时设置的迁移口令"
                          autoComplete="off"
                        />
                      </label>
                      <Button
                        disabled={importBusy || !importPassphrase.trim()}
                        onClick={() =>
                          importPath &&
                          void inspectFile(importPath, importPassphrase)
                        }
                      >
                        解锁加密密钥
                      </Button>
                    </div>
                  ) : importPreview.manifest.hasEncryptedSecrets ? (
                    <InlineNotice tone="info">
                      已解锁 {importPreview.decryptedApiKeyTargets.length}{" "}
                      端加密 API Key，导入时将自动写入并生效。
                    </InlineNotice>
                  ) : null}

                  <div className="fy-transfer-checks">
                    {(
                      [
                        ["importModels", "导入模型配置"],
                        ["importCustomTemplates", "导入自定义模板"],
                        ["importMcp", "导入 MCP 清单"],
                        ["importPrompts", "导入全局提示词"],
                        ["importSkills", "导入 Skills"],
                        ["importSessions", "导入历史会话档案"],
                      ] as const
                    ).map(([key, label]) => (
                      <label key={key} className="fy-transfer-check-row">
                        <Checkbox
                          checked={importFlags[key]}
                          onCheckedChange={(v) =>
                            setImportFlags((prev) => ({
                              ...prev,
                              [key]: Boolean(v),
                            }))
                          }
                          label={label}
                        />
                        <span>{label}</span>
                      </label>
                    ))}
                  </div>
                </>
              ) : (
                <InlineNotice tone="info">
                  选择来自旧电脑或硬件载体的 <code>.fyagent-pack</code>{" "}
                  文件，系统将先校验 SHA-256 完整性再展示导入明细。
                </InlineNotice>
              )}
            </div>

            {importBusy ? (
              <div className="fy-transfer-loading">
                <Spinner label="正在处理迁移包" />
                <span>正在校验或写入配置…</span>
              </div>
            ) : null}
            {importError ? (
              <InlineNotice tone="error">{importError}</InlineNotice>
            ) : null}
            {importPreview?.warnings.map((w, idx) => (
              <InlineNotice key={idx} tone="warning">
                {w}
              </InlineNotice>
            ))}

            {importOutcome ? (
              <>
                <InlineNotice tone="info">
                  导入完成：模型写入 {importOutcome.modelsApplied} 端（跳过{" "}
                  {importOutcome.modelsSkipped} 端）、自定义模板{" "}
                  {importOutcome.templatesImported} 个、MCP{" "}
                  {importOutcome.mcpImported} 项、提示词{" "}
                  {importOutcome.promptsImported} 条、Skills{" "}
                  {importOutcome.skillsImported} 个、会话档案{" "}
                  {importOutcome.sessionsArchived} 条。
                  {importOutcome.sessionArchiveDir ? (
                    <>
                      {" "}
                      档案位置：<code>{importOutcome.sessionArchiveDir}</code>
                    </>
                  ) : null}
                  {importOutcome.nativeSessionPackagePath ? (
                    <>
                      {" "}
                      原生还原包：
                      <code>{importOutcome.nativeSessionPackagePath}</code>
                    </>
                  ) : null}
                </InlineNotice>
                {importOutcome.warnings.map((w, idx) => (
                  <InlineNotice key={`imp-w-${idx}`} tone="warning">
                    {w}
                  </InlineNotice>
                ))}
              </>
            ) : null}
          </div>
        </FeatureTabPanel>

        <FeatureTabPanel
          tabsId={tabsId}
          value="ssh"
          active={tab === "ssh"}
          layout="flow"
        >
          <div className="fy-transfer-body">
            <InlineNotice tone="info">
              免远端安装模式：无需在远程机器安装
              FyAgent，本机直接生成就绪配置文件并通过 SSH
              免密推送，覆盖前自动备份至 <code>~/.fyagent-remote-backups/</code>
              。
            </InlineNotice>

            <div className="fy-transfer-card">
              <strong>1. 选择或填写 SSH 远程主机</strong>
              {sshProfilesQuery.data && sshProfilesQuery.data.length > 0 ? (
                <div className="fy-transfer-chip-row">
                  <span>常用主机：</span>
                  {sshProfilesQuery.data.map((profile) => (
                    <span key={profile.id} className="fy-transfer-profile-chip">
                      <Button
                        className={
                          activeProfileId === profile.id
                            ? "fy-control-button-primary"
                            : undefined
                        }
                        onClick={() => applySavedProfile(profile)}
                      >
                        {profile.name} ({profile.host})
                      </Button>
                      <Button
                        disabled={profileSaving}
                        aria-label={`删除常用主机 ${profile.name}`}
                        onClick={() => void handleDeleteProfile(profile)}
                      >
                        删除
                      </Button>
                    </span>
                  ))}
                </div>
              ) : null}
              {sshHostsQuery.data && sshHostsQuery.data.length > 0 ? (
                <div className="fy-transfer-chip-row">
                  <span>~/.ssh/config 发现：</span>
                  {sshHostsQuery.data.map((entry) => (
                    <Button
                      key={entry.alias}
                      className={
                        !activeProfileId && sshHost === entry.alias
                          ? "fy-control-button-primary"
                          : undefined
                      }
                      onClick={() => applyHostEntry(entry)}
                    >
                      {entry.alias}
                      {entry.hostName ? ` (${entry.hostName})` : ""}
                    </Button>
                  ))}
                </div>
              ) : null}

              <div className="fy-transfer-grid">
                <label className="fy-transfer-field">
                  <span>SSH 主机别名 / IP</span>
                  <Input
                    value={sshHost}
                    onChange={(e) => {
                      setSshHost(e.target.value);
                      resetSshTransientState();
                    }}
                    placeholder="例如 dev-box 或 10.0.0.12"
                  />
                </label>
                <label className="fy-transfer-field">
                  <span>用户名（别名已配置可留空）</span>
                  <Input
                    value={sshUser}
                    onChange={(e) => {
                      setSshUser(e.target.value);
                      resetSshTransientState();
                    }}
                    placeholder="例如 dev 或 alice"
                  />
                </label>
                <label className="fy-transfer-field">
                  <span>端口（默认 22）</span>
                  <Input
                    value={sshPort}
                    onChange={(e) => {
                      setSshPort(e.target.value);
                      resetSshTransientState();
                    }}
                    placeholder="22"
                  />
                </label>
                <label className="fy-transfer-field">
                  <span>私钥路径（选填，默认复用 ssh-agent）</span>
                  <Input
                    value={sshIdentityFile}
                    onChange={(e) => {
                      setSshIdentityFile(e.target.value);
                      resetSshTransientState();
                    }}
                    placeholder="~/.ssh/id_ed25519"
                  />
                </label>
              </div>
              <PathMappingFields
                idPrefix="ssh-map"
                value={sshPathMapping}
                onChange={(next) => {
                  setSshPathMapping(next);
                  resetSshTransientState();
                }}
              />
              <div className="fy-transfer-chip-row">
                <Button disabled={sshBusy} onClick={() => void handleTestSsh()}>
                  测试 SSH 连通性
                </Button>
                <Button
                  disabled={sshBusy || profileSaving}
                  onClick={() => {
                    setProfileName((prev) => prev || sshHost.trim());
                    setSaveProfileOpen((openForm) => !openForm);
                  }}
                >
                  {saveProfileOpen ? "取消保存" : "存为常用主机"}
                </Button>
                {sshTestResult ? (
                  <Badge tone={sshTestResult.ok ? "accent" : "warning"}>
                    {sshTestResult.ok
                      ? `已连通${sshTestResult.remoteHome ? ` · ${sshTestResult.remoteHome}` : ""}`
                      : sshTestResult.message}
                  </Badge>
                ) : null}
              </div>
              {saveProfileOpen ? (
                <div className="fy-transfer-field-row">
                  <label className="fy-transfer-field">
                    <span>
                      常用主机名称（仅存连接信息与目录映射，不保存密钥）
                    </span>
                    <Input
                      value={profileName}
                      onChange={(e) => setProfileName(e.target.value)}
                      placeholder="例如 GPU 开发机"
                    />
                  </label>
                  <Button
                    className="fy-control-button-primary"
                    disabled={profileSaving}
                    onClick={() => void handleSaveProfile()}
                  >
                    {profileSaving ? "正在保存…" : "保存主机档案"}
                  </Button>
                </div>
              ) : null}
            </div>

            <div className="fy-transfer-card">
              <strong>2. 勾选要推送的配置文件</strong>
              <div className="fy-transfer-checks">
                {(
                  [
                    [
                      "includeModels",
                      "CLI 模型配置 (Claude / Codex / OpenCode / omp)",
                    ],
                    ["includeMcp", "MCP 清单"],
                    ["includePrompts", "全局提示词 (CLAUDE.md / AGENTS.md)"],
                    ["includeSkills", "Skills 纯文本目录"],
                  ] as const
                ).map(([key, label]) => (
                  <label key={key} className="fy-transfer-check-row">
                    <Checkbox
                      checked={sshSelection[key]}
                      onCheckedChange={(v) => {
                        setSshSelection((prev) => ({
                          ...prev,
                          [key]: Boolean(v),
                        }));
                        setSshPreview(null);
                        setSshOutcome(null);
                      }}
                      label={label}
                    />
                    <span>{label}</span>
                  </label>
                ))}
              </div>
            </div>

            {sshBusy ? (
              <div className="fy-transfer-loading">
                <Spinner label="正在与远端 SSH 通信" />
                <span>正在处理远端配置文件…</span>
              </div>
            ) : null}
            {sshError ? (
              <InlineNotice tone="error">{sshError}</InlineNotice>
            ) : null}

            {sshPreview && !sshOutcome ? (
              <section
                className="fy-transfer-card"
                aria-label="远端待推送文件预览"
              >
                <div className="fy-transfer-budget-header">
                  <strong>
                    目标 {sshPreview.destination} · 共 {sshPreview.files.length}{" "}
                    个文件（{formatBytes(sshPreview.totalBytes)}）
                  </strong>
                  <Badge tone={sshPreview.reachable ? "accent" : "warning"}>
                    {sshPreview.reachable ? "已探测远端状态" : "未连通预览"}
                  </Badge>
                </div>
                <ul className="fy-transfer-file-list">
                  {sshPreview.files.map((file) => (
                    <li key={file.remotePath} className="fy-transfer-file-row">
                      <span className="fy-transfer-file-path">
                        <code>{file.remotePath}</code>
                        <Badge
                          tone={file.action === "update" ? "warning" : "accent"}
                        >
                          {file.action === "update"
                            ? "备份后更新"
                            : file.action === "create"
                              ? "新建"
                              : "待写入"}
                        </Badge>
                        {file.containsSecret ? (
                          <Badge tone="neutral">chmod {file.mode}</Badge>
                        ) : null}
                        {file.summary ? (
                          <span className="fy-transfer-file-summary">
                            {file.summary}
                          </span>
                        ) : null}
                      </span>
                      <span>{formatBytes(file.bytes)}</span>
                    </li>
                  ))}
                </ul>
                {sshPreview.warnings.map((w, idx) => (
                  <InlineNotice key={idx} tone="warning">
                    {w}
                  </InlineNotice>
                ))}
              </section>
            ) : null}

            {sshOutcome ? (
              <section className="fy-transfer-card" aria-label="SSH 推送结果">
                <InlineNotice tone={sshOutcome.ok ? "info" : "warning"}>
                  {sshOutcome.ok ? "推送已完成" : "部分文件推送失败"} ·
                  远端备份目录：<code>{sshOutcome.backupDir}</code>
                </InlineNotice>
                <ul className="fy-transfer-file-list">
                  {sshOutcome.outcomes.map((item) => (
                    <li key={item.remotePath} className="fy-transfer-file-row">
                      <span className="fy-transfer-file-path">
                        <Badge tone={item.ok ? "accent" : "warning"}>
                          {item.ok ? "已生效" : "失败"}
                        </Badge>
                        <code>{item.remotePath}</code>
                      </span>
                      <span>{item.message}</span>
                    </li>
                  ))}
                </ul>
                {sshOutcome.warnings.map((w, idx) => (
                  <InlineNotice key={`ssh-w-${idx}`} tone="warning">
                    {w}
                  </InlineNotice>
                ))}
              </section>
            ) : null}
          </div>
        </FeatureTabPanel>

        <FeatureTabPanel
          tabsId={tabsId}
          value="wsl"
          active={tab === "wsl"}
          layout="flow"
        >
          <div className="fy-transfer-body">
            {wslStatus?.supported === false ? (
              <InlineNotice tone="info">
                {wslStatus.message ||
                  "WSL 同步仅在 Windows 电脑可用；在 macOS 上请使用「打包迁移包」或「推送到 SSH 远端」。"}
              </InlineNotice>
            ) : (
              <>
                <InlineNotice tone="info">
                  手动同步模式：将本机已接管的 CLI 模型、MCP、提示词与 Skills
                  安全同步至所选 WSL 发行版，覆盖前自动备份至{" "}
                  <code>~/.fyagent-wsl-backups/</code>
                  ；未接管软件（如 TRAE/QoderWork）及官方 OAuth 登录不会同步。
                </InlineNotice>

                <div className="fy-transfer-card">
                  <strong>1. 选择目标 WSL 发行版</strong>
                  {wslDistrosQuery.isLoading ? (
                    <div className="fy-transfer-loading">
                      <Spinner label="正在检测 WSL 发行版" />
                      <span>正在检测本机 WSL 发行版…</span>
                    </div>
                  ) : wslStatus?.distros && wslStatus.distros.length > 0 ? (
                    <div className="fy-transfer-chip-row">
                      <span>已安装发行版：</span>
                      {wslStatus.distros.map((distro) => (
                        <Button
                          key={distro.name}
                          className={
                            effectiveWslDistro === distro.name
                              ? "fy-control-button-primary"
                              : undefined
                          }
                          onClick={() => {
                            setWslDistroInput(distro.name);
                            resetWslTransientState();
                          }}
                        >
                          {distro.name} ({distro.state} · WSL {distro.version}
                          {distro.isDefault ? " · 默认" : ""})
                        </Button>
                      ))}
                    </div>
                  ) : wslStatus?.message ? (
                    <InlineNotice tone="warning">
                      {wslStatus.message}
                    </InlineNotice>
                  ) : null}

                  <div className="fy-transfer-field-row">
                    <label className="fy-transfer-field">
                      <span>发行版名称</span>
                      <Input
                        value={wslDistroInput}
                        onChange={(e) => {
                          setWslDistroInput(e.target.value);
                          resetWslTransientState();
                        }}
                        placeholder={
                          effectiveWslDistro || "输入已安装的发行版名称"
                        }
                      />
                    </label>
                    <Button
                      disabled={wslDistrosQuery.isFetching || wslBusy}
                      onClick={() => void wslDistrosQuery.refetch()}
                    >
                      刷新发行版列表
                    </Button>
                  </div>

                  <PathMappingFields
                    idPrefix="wsl-map"
                    value={wslPathMapping}
                    onChange={(next) => {
                      setWslPathMapping(next);
                      resetWslTransientState();
                    }}
                  />
                </div>

                <div className="fy-transfer-card">
                  <strong>2. 勾选要同步到 WSL 的配置文件</strong>
                  <div className="fy-transfer-checks">
                    {(
                      [
                        [
                          "includeModels",
                          "CLI 模型配置 (Claude / Codex / OpenCode / omp)",
                        ],
                        ["includeMcp", "MCP 清单（自动适配 POSIX 命令）"],
                        [
                          "includePrompts",
                          "全局提示词 (CLAUDE.md / AGENTS.md)",
                        ],
                        ["includeSkills", "Skills 纯文本目录"],
                      ] as const
                    ).map(([key, label]) => (
                      <label key={key} className="fy-transfer-check-row">
                        <Checkbox
                          checked={wslSelection[key]}
                          onCheckedChange={(v) => {
                            setWslSelection((prev) => ({
                              ...prev,
                              [key]: Boolean(v),
                            }));
                            resetWslTransientState();
                          }}
                          label={label}
                        />
                        <span>{label}</span>
                      </label>
                    ))}
                  </div>
                </div>

                {wslBusy ? (
                  <div className="fy-transfer-loading">
                    <Spinner label="正在与 WSL 通信" />
                    <span>正在探测或同步 WSL 配置文件…</span>
                  </div>
                ) : null}
                {wslError ? (
                  <InlineNotice tone="error">{wslError}</InlineNotice>
                ) : null}

                {wslPreview && !wslOutcome ? (
                  <section
                    className="fy-transfer-card"
                    aria-label="WSL 待同步文件预览"
                  >
                    <div className="fy-transfer-budget-header">
                      <strong>
                        目标 {wslPreview.destination} · 共{" "}
                        {wslPreview.files.length} 个文件（
                        {formatBytes(wslPreview.totalBytes)}）
                      </strong>
                      <Badge tone={wslPreview.reachable ? "accent" : "warning"}>
                        {wslPreview.reachable ? "已探测 WSL 状态" : "离线预览"}
                      </Badge>
                    </div>
                    <ul className="fy-transfer-file-list">
                      {wslPreview.files.map((file) => (
                        <li
                          key={file.remotePath}
                          className="fy-transfer-file-row"
                        >
                          <span className="fy-transfer-file-path">
                            <code>{file.remotePath}</code>
                            <Badge
                              tone={
                                file.action === "update" ? "warning" : "accent"
                              }
                            >
                              {file.action === "update"
                                ? "备份后更新"
                                : file.action === "create"
                                  ? "新建"
                                  : "待写入"}
                            </Badge>
                            {file.containsSecret ? (
                              <Badge tone="neutral">chmod {file.mode}</Badge>
                            ) : null}
                            {file.summary ? (
                              <span className="fy-transfer-file-summary">
                                {file.summary}
                              </span>
                            ) : null}
                          </span>
                          <span>{formatBytes(file.bytes)}</span>
                        </li>
                      ))}
                    </ul>
                    {wslPreview.warnings.map((w, idx) => (
                      <InlineNotice key={idx} tone="warning">
                        {w}
                      </InlineNotice>
                    ))}
                  </section>
                ) : null}

                {wslOutcome ? (
                  <section
                    className="fy-transfer-card"
                    aria-label="WSL 同步结果"
                  >
                    <InlineNotice tone={wslOutcome.ok ? "info" : "warning"}>
                      {wslOutcome.ok
                        ? "WSL 同步已完成"
                        : "部分文件同步失败（已自动回滚）"}{" "}
                      · 备份目录：<code>{wslOutcome.backupDir}</code>
                    </InlineNotice>
                    <ul className="fy-transfer-file-list">
                      {wslOutcome.outcomes.map((item) => (
                        <li
                          key={item.remotePath}
                          className="fy-transfer-file-row"
                        >
                          <span className="fy-transfer-file-path">
                            <Badge tone={item.ok ? "accent" : "warning"}>
                              {item.ok ? "已生效" : "失败"}
                            </Badge>
                            <code>{item.remotePath}</code>
                          </span>
                          <span>{item.message}</span>
                        </li>
                      ))}
                    </ul>
                    {wslOutcome.warnings.map((w, idx) => (
                      <InlineNotice key={`wsl-w-${idx}`} tone="warning">
                        {w}
                      </InlineNotice>
                    ))}
                  </section>
                ) : null}
              </>
            )}
          </div>
        </FeatureTabPanel>
      </div>
    </Dialog>
  );
}

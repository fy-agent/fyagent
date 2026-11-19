import {
  lazy,
  Suspense,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import type { ProviderApiFormFill } from "../../shared/features/controls/ProviderApiFields";
import { getAgentBrand, type AgentIconId } from "../../shared/assets/agents";
import {
  isOfficialOnlyModelTarget,
  type ProviderAppId,
} from "../../shared/features/types";
import { Button } from "../../shared/ui/Button";
import {
  CatalogList,
  CatalogListItem,
  CatalogMasterDetail,
  CatalogRail,
} from "../../shared/ui/catalog";
import {
  usePersistentSearchParams,
  useStickyVisibleValue,
} from "../../shared/ui/usePersistentSearchParams";
import {
  MODEL_TARGETS,
  parseExplicitModelTarget,
  type ModelTarget,
} from "./quickSetup";
import { OmpModelsPanel } from "./OmpModelsPanel";
import { OpenCodeModelsPanel } from "./OpenCodeModelsPanel";
import { ProviderPanel } from "./ProviderPanel";
import { QoderModelsPanel } from "./QoderModelsPanel";
import { TraeModelsPanel } from "./TraeModelsPanel";
import { WorkBuddyPanel } from "./WorkBuddyPanel";
import { ConfigPackButton } from "../../shared/features/config-pack-ui/ConfigPackButton";
import type { PortableProvider } from "../../domain/config-pack";
import type { SaveCustomGatewayTemplateRequest } from "../../domain/configuration/presets/desktopPresetTemplates";
import "./Page.css";

const PresetBatchDrawer = lazy(() =>
  import("./PresetBatchDrawer").then((mod) => ({
    default: mod.PresetBatchDrawer,
  })),
);

const ModelHealthDialog = lazy(() =>
  import("./ModelHealthDialog").then((mod) => ({
    default: mod.ModelHealthDialog,
  })),
);

const TARGET_LABELS: Record<ModelTarget, string> = {
  qoderwork: "QoderWork CN",
  trae: "TRAE Work CN",
  workbuddy: "WorkBuddy",
  grokbuild: "Grok Build",
  codex: "Codex",
  claude: "Claude Code",
  opencode: "OpenCode",
  omp: "Oh My Pi (omp)",
};

const TARGET_ICON_IDS: Readonly<Record<ModelTarget, AgentIconId>> = {
  qoderwork: "qoderwork",
  trae: "trae-work",
  workbuddy: "workbuddy",
  grokbuild: "grokbuild",
  codex: "codex",
  claude: "claude-code",
  opencode: "opencode",
  omp: "omp",
};

function ModelsTargetPanel({
  target,
  active,
  blockedProviderWrites,
  onBlockProviderWrites,
  onRecoverProviderWrites,
  importedForm,
}: {
  target: ModelTarget;
  active: boolean;
  blockedProviderWrites: Partial<Record<ProviderAppId | "opencode", boolean>>;
  onBlockProviderWrites: (app: ProviderAppId | "opencode") => void;
  onRecoverProviderWrites: (app: ProviderAppId) => void;
  importedForm: {
    app: "claude" | "codex";
    sequence: number;
    form: ProviderApiFormFill;
  } | null;
}) {
  switch (target) {
    case "workbuddy":
      return <WorkBuddyPanel active={active} />;
    case "codex":
    case "claude":
    case "grokbuild":
      return (
        <ProviderPanel
          key={`${target}:${importedForm?.app === target ? importedForm.sequence : 0}`}
          app={target}
          active={active}
          writesBlocked={Boolean(blockedProviderWrites[target])}
          onBlockWrites={onBlockProviderWrites}
          onRecoverWrites={onRecoverProviderWrites}
          initialForm={
            importedForm?.app === target ? importedForm.form : undefined
          }
        />
      );
    case "qoderwork":
      return <QoderModelsPanel />;
    case "trae":
      return <TraeModelsPanel active={active} />;
    case "opencode":
      return (
        <OpenCodeModelsPanel
          active={active}
          writesBlocked={Boolean(blockedProviderWrites.opencode)}
          onBlockWrites={() => onBlockProviderWrites("opencode")}
        />
      );
    case "omp":
      return <OmpModelsPanel active={active} />;
  }
}

export function ModelsPage() {
  const pageMountedRef = useRef(true);
  useEffect(() => {
    pageMountedRef.current = true;
    return () => {
      pageMountedRef.current = false;
    };
  }, []);
  const { visible, searchParams, setSearchParams } =
    usePersistentSearchParams();
  const [blockedProviderWrites, setBlockedProviderWrites] = useState<
    Partial<Record<ProviderAppId | "opencode", boolean>>
  >({});
  const [batchDrawerOpen, setBatchDrawerOpen] = useState(false);
  const [batchDrawerSeed, setBatchDrawerSeed] = useState<{
    key: number;
    draft: Partial<SaveCustomGatewayTemplateRequest> | null;
  }>({ key: 0, draft: null });
  const batchButtonOriginRef = useRef<HTMLElement | null>(null);
  const [healthDialogOpen, setHealthDialogOpen] = useState(false);
  const healthButtonOriginRef = useRef<HTMLElement | null>(null);
  const [importedForm, setImportedForm] = useState<{
    app: "claude" | "codex";
    sequence: number;
    form: ProviderApiFormFill;
  } | null>(null);
  const fillImportedForm = (provider: Readonly<PortableProvider>) => {
    setImportedForm((current) => ({
      app: provider.app,
      sequence: (current?.sequence ?? 0) + 1,
      form: {
        name: provider.name,
        connection: {
          baseUrl: provider.endpoint,
          modelId: provider.model,
          protocol: provider.wireApi ?? "anthropic",
        },
      },
    }));
    setSearchParams({ target: provider.app });
  };
  const rawTarget = searchParams.get("target");
  const target = useStickyVisibleValue(
    visible,
    parseExplicitModelTarget(rawTarget),
    "claude",
  );
  const targets = useMemo(() => MODEL_TARGETS, []);

  const blockProviderWrites = useCallback((app: ProviderAppId | "opencode") => {
    if (!pageMountedRef.current) return;
    setBlockedProviderWrites((current) => ({
      ...current,
      [app]: true,
    }));
  }, []);
  const recoverProviderWrites = (app: ProviderAppId) => {
    setBlockedProviderWrites((current) => ({ ...current, [app]: false }));
  };

  return (
    <div
      className="fy-feature-page fy-split-page fy-catalog-page fy-models-page"
      data-testid="models-page"
      aria-label="模型"
    >
      <header className="fy-models-page-heading">
        <h1>模型管理</h1>
        <div className="fy-models-heading-actions">
          <Button
            className="fy-control-button-primary"
            onClick={(e) => {
              batchButtonOriginRef.current = e.currentTarget;
              setBatchDrawerSeed((prev) => ({
                key: prev.key + 1,
                draft: null,
              }));
              setBatchDrawerOpen(true);
            }}
          >
            一键配置主流大模型
          </Button>
          <Button
            onClick={(e) => {
              healthButtonOriginRef.current = e.currentTarget;
              setHealthDialogOpen(true);
            }}
          >
            全端体检与克隆
          </Button>
          <ConfigPackButton onFillModelForm={fillImportedForm} />
        </div>
      </header>
      {batchDrawerOpen ? (
        <Suspense fallback={null}>
          <PresetBatchDrawer
            key={batchDrawerSeed.key}
            open={batchDrawerOpen}
            originRef={batchButtonOriginRef}
            onOpenChange={setBatchDrawerOpen}
            initialTemplateDraft={batchDrawerSeed.draft}
          />
        </Suspense>
      ) : null}
      {healthDialogOpen ? (
        <Suspense fallback={null}>
          <ModelHealthDialog
            open={healthDialogOpen}
            originRef={healthButtonOriginRef}
            onOpenChange={setHealthDialogOpen}
            onSaveAsTemplate={(draft) => {
              setBatchDrawerSeed((prev) => ({ key: prev.key + 1, draft }));
              setBatchDrawerOpen(true);
            }}
          />
        </Suspense>
      ) : null}
      <CatalogMasterDetail>
        <CatalogRail as="aside" ariaLabel="模型配置目标" title="选择应用">
          <CatalogList>
            {targets.map((candidate) => {
              const officialOnly = isOfficialOnlyModelTarget(candidate);
              return (
                <CatalogListItem
                  key={candidate}
                  asset={getAgentBrand(TARGET_ICON_IDS[candidate])}
                  label={TARGET_LABELS[candidate]}
                  summary={officialOnly ? "仅支持官方模型" : undefined}
                  muted={officialOnly}
                  selected={candidate === target}
                  testId={`model-target-${candidate}`}
                  onSelect={() =>
                    setSearchParams(
                      (current) => {
                        const next = new URLSearchParams(current);
                        next.set("target", candidate);
                        return next;
                      },
                      { replace: true },
                    )
                  }
                />
              );
            })}
          </CatalogList>
        </CatalogRail>
        <div className="fy-models-target-stack">
          <ModelsTargetPanel
            target={target}
            active={visible}
            blockedProviderWrites={blockedProviderWrites}
            onBlockProviderWrites={blockProviderWrites}
            onRecoverProviderWrites={recoverProviderWrites}
            importedForm={importedForm}
          />
        </div>
      </CatalogMasterDetail>
    </div>
  );
}

import { lazy, Suspense, useEffect, useRef } from "react";

import { PRODUCT_DIRECTORY } from "../../shared/features/directory";
import {
  useAgentCatalog,
  useFirstUseGuideState,
} from "../../shared/features/queries";
import { useFrontendReady } from "../../shared/platform/useFrontendReady";
import { usePersistentSearchParams } from "../../shared/ui/usePersistentSearchParams";
import { Button } from "../../shared/ui/Button";
import { EmptyState, InlineNotice, Spinner } from "../../shared/ui/primitives";

import { AgentConfiguration } from "./AgentConfiguration";
import { AgentDirectory } from "./AgentDirectory";
import { AGENT_SECTION_IDS, type AgentSection } from "./agentSections";
import { useAgentDirectoryScan } from "./useAgentDirectoryScan";
import "./Page.css";

const FirstUseGuide = lazy(() =>
  import("./FirstUseGuide").then((module) => ({
    default: module.FirstUseGuide,
  })),
);

function agentSection(value: string | null): AgentSection | null {
  return AGENT_SECTION_IDS.find((section) => section === value) ?? null;
}

export function AgentsPage() {
  const { visible, searchParams, setSearchParams } =
    usePersistentSearchParams();
  const catalogQuery = useAgentCatalog();
  const entries = catalogQuery.data?.agents ?? [];
  const rawTarget = searchParams.get("target");
  const rawSection = searchParams.get("section");
  const directoryEntry = PRODUCT_DIRECTORY.find(
    (entry) => entry.agentId === rawTarget,
  );
  const requestedSection = agentSection(rawSection);
  const section = requestedSection ?? "models";
  const catalogEntry = directoryEntry
    ? entries.find((entry) => entry.id === directoryEntry.agentId)
    : undefined;
  const showDirectory = !directoryEntry;
  const guideQuery = useFirstUseGuideState(showDirectory);
  const guideLoading = showDirectory && guideQuery.isPending;
  const showGuide = showDirectory && guideQuery.data === "pending";
  const headingRef = useRef<HTMLHeadingElement>(null);
  const guideWasShown = useRef(false);
  useFrontendReady(
    !catalogQuery.isPending &&
      !guideLoading &&
      (!showGuide || entries.length === 0),
  );
  const scanController = useAgentDirectoryScan({
    autoStart: true,
    active: visible && !guideLoading && !showGuide,
  });

  useEffect(() => {
    if (!visible || !showDirectory) return;
    if (showGuide) {
      guideWasShown.current = true;
    } else if (
      !guideLoading &&
      !catalogQuery.isPending &&
      guideWasShown.current
    ) {
      guideWasShown.current = false;
      headingRef.current?.focus({ preventScroll: true });
    }
  }, [catalogQuery.isPending, guideLoading, showDirectory, showGuide, visible]);

  useEffect(() => {
    if (!visible) return;
    if (!rawTarget) {
      if (rawSection) setSearchParams({}, { replace: true });
      return;
    }
    if (!directoryEntry) {
      setSearchParams({}, { replace: true });
      return;
    }
    if (!requestedSection) {
      setSearchParams(
        { target: directoryEntry.agentId, section: "models" },
        { replace: true },
      );
    }
  }, [
    directoryEntry,
    rawSection,
    rawTarget,
    requestedSection,
    setSearchParams,
    visible,
  ]);

  const returnToDirectory = () => {
    setSearchParams({});
  };

  return (
    <div
      className="fy-feature-page fy-agents-page"
      data-testid="agents-page"
      data-view={
        showGuide ? "guide" : showDirectory ? "directory" : "configuration"
      }
      aria-label="AI 软件配置"
    >
      {catalogQuery.error && catalogQuery.data !== undefined ? (
        <InlineNotice tone="warning">
          暂时无法刷新 Agent 目录，正在显示已加载内容。
        </InlineNotice>
      ) : null}

      {catalogQuery.isPending || guideLoading ? (
        <EmptyState title="正在加载 Agent 目录">
          <Spinner label="正在加载 Agent 目录" />
        </EmptyState>
      ) : catalogQuery.isError && catalogQuery.data === undefined ? (
        <EmptyState
          title="无法加载 Agent 目录"
          description="暂时无法获取应用信息，请重试。"
          actions={
            <Button onClick={() => void catalogQuery.refetch()}>重试</Button>
          }
        />
      ) : entries.length === 0 ? (
        <EmptyState
          title="Agent 目录暂不可用"
          description="暂时没有可显示的应用，请重试。"
          actions={
            <Button onClick={() => void catalogQuery.refetch()}>重试</Button>
          }
        />
      ) : directoryEntry && !catalogEntry ? (
        <EmptyState
          title="无法恢复 Agent 配置"
          description="当前目录没有这个 Agent，请返回软件目录后重试。"
          actions={<Button onClick={returnToDirectory}>返回目录</Button>}
        />
      ) : showGuide ? (
        <Suspense
          fallback={
            <EmptyState title="正在加载引导">
              <Spinner label="正在加载引导" />
            </EmptyState>
          }
        >
          <FirstUseGuide entries={entries} />
        </Suspense>
      ) : showDirectory ? (
        <AgentDirectory
          headingRef={headingRef}
          entries={entries}
          scanController={scanController}
          onConfigure={(agentId) =>
            setSearchParams({ target: agentId, section: "models" })
          }
        />
      ) : catalogEntry && directoryEntry ? (
        <AgentConfiguration
          entry={directoryEntry}
          catalogEntry={catalogEntry}
          section={section}
          onBack={returnToDirectory}
          onSectionChange={(nextSection) =>
            setSearchParams({
              target: directoryEntry.agentId,
              section: nextSection,
            })
          }
        />
      ) : null}
    </div>
  );
}

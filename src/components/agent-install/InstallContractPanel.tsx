import { useTranslation } from "react-i18next";
import type { InstallContract, LayerState } from "@/types/agentInstall";
import { Button } from "@/components/ui/button";

const COLORS: Record<LayerState, string> = {
  ok: "#2F6B46",
  warn: "#9C6A0B",
  fail: "#C62828",
  unknown: "#546E7A",
};

function Card({
  title,
  state,
  children,
}: {
  title: string;
  state: LayerState;
  children: React.ReactNode;
}) {
  return (
    <section className="rounded-xl border border-border/60 p-4 space-y-2">
      <header className="flex items-center justify-between gap-2">
        <h3 className="text-sm font-semibold">{title}</h3>
        <span className="text-xs font-medium" style={{ color: COLORS[state] }}>
          {state}
        </span>
      </header>
      <div className="text-sm text-muted-foreground space-y-1">{children}</div>
    </section>
  );
}

interface InstallContractPanelProps {
  contract: InstallContract;
  onOpenGuide: () => void;
  onRecheck: () => void;
  onRegenerate: () => void;
  onInstall: () => void;
}

export function InstallContractPanel({
  contract,
  onOpenGuide,
  onRecheck,
  onRegenerate,
  onInstall,
}: InstallContractPanelProps) {
  const { t } = useTranslation();
  const stale = contract.plan.snapshotStale;
  const summary = stale
    ? t("agentInstall.needsReconfirm")
    : contract.installAllowed
      ? t("agentInstall.installable")
      : t("agentInstall.notInstallable");

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <p className="text-sm font-medium">{summary}</p>
        {contract.package.integrityState === "warn" ? (
          <p className="text-xs" style={{ color: COLORS.warn }}>
            {t("agentInstall.warnContinue")}
          </p>
        ) : null}
      </div>
      <div className="grid gap-3 md:grid-cols-2">
        <Card title={t("agentInstall.source")} state={contract.catalog.sourceState}>
          <p>{contract.catalog.legalEntity}</p>
          <p>{contract.catalog.licenseScope}</p>
          <p>{contract.catalog.checkedAt}</p>
        </Card>
        <Card
          title={t("agentInstall.integrity")}
          state={contract.package.integrityState}
        >
          <p>{contract.package.integritySummary}</p>
          <p>{contract.package.checkedAt}</p>
        </Card>
        <Card
          title={t("agentInstall.preflight")}
          state={contract.environment.preflightState}
        >
          <p>{contract.environment.checkedAt}</p>
          {contract.environment.preflightState === "unknown" ? (
            <p>{t("agentInstall.unknownBlocks")}</p>
          ) : null}
        </Card>
        <Card title={t("agentInstall.plan")} state={stale ? "fail" : "ok"}>
          <p>{contract.plan.planSnapshotId ?? "—"}</p>
          {stale ? <p>{t("agentInstall.stalePlan")}</p> : null}
        </Card>
      </div>
      <div className="flex flex-wrap gap-2">
        <Button variant="outline" onClick={onOpenGuide} disabled={!contract.guideAllowed}>
          {t("agentInstall.openGuide")}
        </Button>
        <Button variant="outline" onClick={onRecheck}>
          {t("agentInstall.recheck")}
        </Button>
        <Button variant="outline" onClick={onRegenerate}>
          {t("agentInstall.regeneratePlan")}
        </Button>
        <Button
          onClick={onInstall}
          disabled={!contract.installAllowed || stale}
        >
          {stale ? t("agentInstall.regeneratePlan") : t("agentInstall.install")}
        </Button>
      </div>
    </div>
  );
}

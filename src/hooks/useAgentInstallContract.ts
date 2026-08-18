import { useCallback, useEffect, useState } from "react";
import { agentInstallApi } from "@/lib/api/agent-install";
import type { InstallContract, SourceLayerState } from "@/types/agentInstall";

export function useAgentInstallContract() {
  const [catalog, setCatalog] = useState<SourceLayerState[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [contract, setContract] = useState<InstallContract | null>(null);
  const [error, setError] = useState<string | null>(null);

  const loadCatalog = useCallback(async () => {
    const entries = await agentInstallApi.listCatalog();
    setCatalog(entries);
    setSelectedId((current) => current ?? "codex-cli");
  }, []);

  const loadContract = useCallback(async (agentId: string) => {
    setError(null);
    try {
      const next = await agentInstallApi.getContract(agentId);
      setContract(next);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "contract_failed");
    }
  }, []);

  useEffect(() => {
    void loadCatalog();
  }, [loadCatalog]);

  useEffect(() => {
    if (selectedId) {
      void loadContract(selectedId);
    }
  }, [loadContract, selectedId]);

  const createPlan = useCallback(
    async (agentId: string) => {
      const plan = await agentInstallApi.createPlan(agentId);
      setContract((current) =>
        current
          ? {
              ...current,
              plan,
              installAllowed: current.installAllowed && !plan.snapshotStale,
            }
          : current,
      );
    },
    [],
  );

  return {
    catalog,
    selectedId,
    setSelectedId,
    contract,
    error,
    reload: loadCatalog,
    reloadContract: loadContract,
    startInstall: agentInstallApi.startInstall,
    openGuide: agentInstallApi.openOfficialGuide,
    createPlan,
  };
}

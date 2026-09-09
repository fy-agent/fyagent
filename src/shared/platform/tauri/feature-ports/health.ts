import { invoke } from "@tauri-apps/api/core";
import * as z from "zod/mini";

import {
  AGENT_CATALOG_IDS,
  type AgentCatalogId,
} from "../../../features/directory";
import {
  HEALTH_ACTIONS,
  HEALTH_CHECK_IDS,
  HEALTH_CHECK_STATES,
  HEALTH_REASON_CODES,
  HEALTH_SEVERITIES,
  type AgentHealthSnapshot,
  type HealthPort,
} from "../../../features/health";

const HEALTH_READ_ERROR = "无法确认运行状态，请重新检查";
const agentIdSchema = z.enum(AGENT_CATALOG_IDS);

// Health accepts only UTC RFC3339 timestamps emitted by chrono, not the
// larger general-purpose ISO formats. The round trip rejects invalid dates
// (Date.parse alone normalizes e.g. February 30 instead of rejecting it).
const timestamp = z.string().check(
  z.regex(
    /^\d{4}-(?:0[1-9]|1[0-2])-(?:0[1-9]|[12]\d|3[01])T(?:[01]\d|2[0-3]):[0-5]\d:[0-5]\d(?:\.\d{1,9})?(?:Z|\+00:00)$/u,
  ),
  z.refine(
    (value) =>
      Number.isFinite(Date.parse(value)) &&
      new Date(value).toISOString().slice(0, 10) === value.slice(0, 10),
  ),
);
const safeValue = z.string().check(
  z.minLength(1),
  z.maxLength(80),
  z.regex(/^[\p{L}\p{N} ._+()/-]+$/u),
  z.refine(
    (value) =>
      !/^(?:[/.~]|[a-z]:)|(?:sk-|xai-|eyJ|bearer|token|secret)|(?:^|\s)(?:Users|home|private)\//iu.test(
        value,
      ),
  ),
);
const checkSchema = z
  .strictObject({
    id: z.enum(HEALTH_CHECK_IDS),
    state: z.enum(HEALTH_CHECK_STATES),
    severity: z.enum(HEALTH_SEVERITIES),
    reasonCode: z.enum(HEALTH_REASON_CODES),
    checkedAt: timestamp,
    evidenceAt: z.nullable(timestamp),
    value: z.nullable(safeValue),
    action: z.nullable(z.enum(HEALTH_ACTIONS)),
  })
  .check(
    z.refine(
      (check) =>
        check.value === null ||
        check.id === "installation" ||
        check.id === "model",
    ),
    z.refine(
      (check) =>
        check.state !== "not_supported" ||
        (check.severity === "info" && check.action === null),
    ),
    z.refine(
      (check) =>
        check.evidenceAt === null ||
        Date.parse(check.evidenceAt) <= Date.parse(check.checkedAt),
    ),
  );
const snapshotSchema = z
  .strictObject({
    contractVersion: z.literal(1),
    agentId: agentIdSchema,
    checkedAt: timestamp,
    checks: z.array(checkSchema).check(
      z.length(HEALTH_CHECK_IDS.length),
      z.refine(
        (checks) =>
          new Set(checks.map((check) => check.id)).size ===
          HEALTH_CHECK_IDS.length,
      ),
    ),
  })
  .check(
    z.refine((snapshot) =>
      snapshot.checks.every(
        (check) =>
          Date.parse(check.checkedAt) <= Date.parse(snapshot.checkedAt),
      ),
    ),
  );

export function parseAgentHealthSnapshot(
  value: unknown,
  agentId: AgentCatalogId,
): AgentHealthSnapshot {
  const result = snapshotSchema.safeParse(value);
  if (
    !result.success ||
    result.data.agentId !== agentId ||
    Date.parse(result.data.checkedAt) > Date.now() + 60_000
  ) {
    throw new Error(HEALTH_READ_ERROR);
  }
  return result.data;
}

export function createHealthPort(): HealthPort {
  return {
    get: async (agentId) => {
      const parsedId = agentIdSchema.safeParse(agentId);
      if (!parsedId.success) throw new Error("请选择支持的软件");
      try {
        return parseAgentHealthSnapshot(
          await invoke<unknown>("get_agent_health", { agentId: parsedId.data }),
          parsedId.data,
        );
      } catch {
        // Neither rejected IPC payloads nor native errors are safe UI copy.
        throw new Error(HEALTH_READ_ERROR);
      }
    },
  };
}

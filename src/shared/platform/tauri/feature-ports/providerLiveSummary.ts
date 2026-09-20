import {
  apiProtocolsForTarget,
  isApiProtocol,
} from "../../../../domain/configuration/providerApi";
import type {
  ProviderAppId,
  ProviderLiveConnection,
  ProviderLiveSummary,
} from "../../../features/models";
import { hasExactKeys, isRecord } from "./validation";

function isPublicText(value: unknown, limit: number): value is string {
  return (
    typeof value === "string" &&
    value.trim().length > 0 &&
    value.length <= limit &&
    Array.from(value).every((character) => {
      const code = character.charCodeAt(0);
      return code >= 0x20 && (code < 0x7f || code > 0x9f);
    })
  );
}

function parseConnection(
  value: unknown,
  target: ProviderAppId,
): ProviderLiveConnection | null {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, ["baseUrl", "modelId", "protocol"]) ||
    (value.baseUrl !== null && !isPublicText(value.baseUrl, 2048)) ||
    (value.modelId !== null && !isPublicText(value.modelId, 256)) ||
    (value.protocol !== null &&
      (!isApiProtocol(value.protocol) ||
        !apiProtocolsForTarget(target).includes(value.protocol))) ||
    (value.baseUrl === null && value.modelId === null)
  )
    return null;
  if (typeof value.baseUrl === "string") {
    try {
      const url = new URL(value.baseUrl);
      if (
        !["http:", "https:"].includes(url.protocol) ||
        !url.hostname ||
        url.username ||
        url.password ||
        url.search ||
        url.hash
      )
        return null;
    } catch {
      return null;
    }
  }
  return {
    baseUrl: value.baseUrl,
    modelId: value.modelId,
    protocol: value.protocol,
  };
}

/** Live read failures never invalidate an independently valid saved-source list. */
export function parseProviderLiveSummary(
  value: unknown,
  target: ProviderAppId,
): ProviderLiveSummary {
  const unavailable: ProviderLiveSummary = {
    target,
    state: "unreadable",
    exists: null,
    connection: null,
  };
  if (
    !isRecord(value) ||
    !hasExactKeys(value, ["target", "state", "exists", "connection"]) ||
    value.target !== target
  )
    return unavailable;
  if (value.state === "configured" && value.exists === true) {
    const connection = parseConnection(value.connection, target);
    return connection
      ? { target, state: "configured", exists: true, connection }
      : unavailable;
  }
  if (value.connection !== null) return unavailable;
  if (value.state === "missing" && value.exists === false)
    return { target, state: "missing", exists: false, connection: null };
  if (value.state === "not_configured" && value.exists === true)
    return { target, state: "not_configured", exists: true, connection: null };
  if (
    value.state === "unreadable" &&
    (value.exists === null || typeof value.exists === "boolean")
  )
    return {
      target,
      state: "unreadable",
      exists: value.exists,
      connection: null,
    };
  return unavailable;
}

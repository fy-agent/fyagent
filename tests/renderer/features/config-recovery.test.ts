import { describe, expect, it } from "vitest";

import {
  assertConfigRecoveryRequest,
  assertConfigRecoveryTargets,
  parseConfigRecoveryList,
  parseConfigRecoverySnapshot,
} from "@/shared/features/config-recovery";
import { parseManagedAuthConnectionPreview } from "@/shared/features/managed-auth";
import {
  CODEX_CONNECTION_ID,
  CONNECTION_REVISION,
  OPENAI_ACCOUNT_ID,
  connectionPreviewFixture,
} from "../fixtures/managedAuth";

const receiptId = "323e4567-e89b-42d3-a456-426614174000";
const writeTarget = {
  path: "~/.codex/auth.json",
  backupPath: "~/.codex/auth.json.fyagent.backup",
  exists: true,
};
const snapshot = {
  contractVersion: 1,
  target: "codex_auth",
  writeTarget,
  state: "available",
  receiptId,
  restoresExistingFile: true,
};

describe("configuration recovery boundary", () => {
  it("requires closed targets and receipts rather than accepting caller paths", () => {
    expect(
      assertConfigRecoveryRequest({ target: "codex_auth", receiptId }),
    ).toEqual({ target: "codex_auth", receiptId });
    for (const request of [
      { target: "../auth.json", receiptId },
      { target: "codex_auth", receiptId: "stale" },
      { target: "codex_auth", receiptId, path: "/etc/passwd" },
    ])
      expect(() => assertConfigRecoveryRequest(request)).toThrow(
        "文件恢复请求无效",
      );
    for (const targets of [[], ["codex_auth", "codex_auth"], ["unknown"]]) {
      expect(() => assertConfigRecoveryTargets(targets)).toThrow();
    }
  });

  it("binds exact response resources and rejects sensitive or inconsistent fields", () => {
    expect(parseConfigRecoveryList([snapshot], ["codex_auth"])).toEqual([
      snapshot,
    ]);
    for (const invalid of [
      { ...snapshot, accessToken: "secret" },
      { ...snapshot, state: "conflict" },
      { ...snapshot, receiptId: null },
      { ...snapshot, contractVersion: 2 },
      { ...snapshot, writeTarget: { ...writeTarget, contents: "secret" } },
    ])
      expect(() => parseConfigRecoverySnapshot(invalid)).toThrow();
    expect(() =>
      parseConfigRecoveryList([snapshot], ["codex_config"]),
    ).toThrow();
    expect(() =>
      parseConfigRecoveryList(
        [snapshot, snapshot],
        ["codex_auth", "codex_config"],
      ),
    ).toThrow();
    expect(
      parseConfigRecoverySnapshot({ ...snapshot, restoresExistingFile: false })
        .restoresExistingFile,
    ).toBe(false);
  });

  it("accepts file-impact metadata but rejects credentials in an Auth preview", () => {
    const request = {
      connectionId: CODEX_CONNECTION_ID,
      expectedRevision: CONNECTION_REVISION,
      action: "connect_account" as const,
      accountId: OPENAI_ACCOUNT_ID,
    };
    const preview = connectionPreviewFixture(request);
    expect(parseManagedAuthConnectionPreview(preview, request)).toEqual(
      preview,
    );
    expect(() =>
      parseManagedAuthConnectionPreview(
        { ...preview, auth: { token: "secret" } },
        request,
      ),
    ).toThrow();
    expect(() =>
      parseManagedAuthConnectionPreview(
        { ...preview, writeTargets: [{ ...writeTarget, contents: "secret" }] },
        request,
      ),
    ).toThrow();
    expect(() =>
      parseManagedAuthConnectionPreview(preview, {
        ...request,
        accountId: null,
      }),
    ).toThrow();
  });
});

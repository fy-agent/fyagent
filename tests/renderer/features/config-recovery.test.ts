import { describe, expect, it } from "vitest";

import { parseManagedAuthConnectionPreview } from "@/shared/features/managed-auth";
import {
  CODEX_CONNECTION_ID,
  CONNECTION_REVISION,
  OPENAI_ACCOUNT_ID,
  connectionPreviewFixture,
} from "../fixtures/managedAuth";

const writeTarget = {
  path: "~/.codex/auth.json",
  backupPath: "~/.codex/auth.json.fyagent.backup",
  exists: true,
};
describe("authentication file-impact boundary", () => {
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

import { describe, expect, it } from "vitest";

import {
  FORBIDDEN_SEMANTIC_FIELDS_V1,
  SecretContractDecodeError,
  assertPublicNoValue,
  credentialBrowserFixtures,
  decodeBeginSecretCaptureRequest,
  decodeCredentialsSnapshot,
  decodeSecretRef,
} from "@/v2/shared/data/credentials";

const validRequest = {
  captureIntentId: "sci_dddddddddddd4ddd8ddddddddddd5005",
  backendInstanceId: "sbi_cccccccccccc4ccc8ccccccccccc1001",
};

describe("credentials decoder", () => {
  it("rejects every forbidden semantic key", () => {
    for (const key of FORBIDDEN_SEMANTIC_FIELDS_V1) {
      expect(() => assertPublicNoValue({ [key]: "neutral" }), key).toThrow(
        SecretContractDecodeError,
      );
    }
    expect(() => assertPublicNoValue({ apiKey: "neutral" })).toThrow(
      /forbidden semantic field/i,
    );
    expect(() => assertPublicNoValue({ openaiApiKey: "neutral" })).toThrow(
      SecretContractDecodeError,
    );
    expect(() => assertPublicNoValue({ password: "neutral" })).toThrow(
      SecretContractDecodeError,
    );
    expect(() => assertPublicNoValue({ token: "neutral" })).toThrow(
      SecretContractDecodeError,
    );
    expect(() => assertPublicNoValue({ credential: "neutral" })).toThrow(
      SecretContractDecodeError,
    );
  });

  it("rejects API-key shaped strings", () => {
    expect(() => assertPublicNoValue({ displayName: "sk-demo" })).toThrow(
      /credential-shaped/i,
    );
    expect(() => assertPublicNoValue({ displayName: "ghp_demo" })).toThrow(
      /credential-shaped/i,
    );
    expect(() => assertPublicNoValue({ displayName: "Bearer abc" })).toThrow(
      /credential-shaped/i,
    );
  });

  it("rejects secretRefDisplay used as identity", () => {
    expect(() => decodeSecretRef("sec_…ab12")).toThrow(
      /secretRefDisplay must not be used as identity/,
    );
    expect(() =>
      decodeBeginSecretCaptureRequest({
        ...validRequest,
        secretRefDisplay: "sec_…ab12",
      }),
    ).toThrow(/captureIntentId and backendInstanceId/);
  });

  it("never accepts a raw SecretRef on the panel capture request", () => {
    expect(() =>
      decodeBeginSecretCaptureRequest({
        ...validRequest,
        secretRef: "sec_1111111111114111811111111111ab12",
      }),
    ).toThrow(/captureIntentId and backendInstanceId/);
    expect(decodeBeginSecretCaptureRequest(validRequest)).toEqual(validRequest);
  });

  it("rejects unknown fields on public summaries", () => {
    const [ready] = credentialBrowserFixtures.owners;
    expect(() =>
      decodeCredentialsSnapshot({
        ...credentialBrowserFixtures,
        owners: [{ ...ready, extraField: "nope" }],
      }),
    ).toThrow(/unknown field "extraField"/);
  });

  it("accepts the browser public snapshot", () => {
    const decoded = decodeCredentialsSnapshot(
      JSON.parse(JSON.stringify(credentialBrowserFixtures)),
    );
    expect(decoded.owners).toHaveLength(credentialBrowserFixtures.owners.length);
    expect(decoded.secretDeleteImpact.impact.noFallback).toBe(true);
  });
});

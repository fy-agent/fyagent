import { describe, expect, it } from "vitest";

import {
  OMP_SUGGESTED_NEW_PROVIDER_ID,
  buildOmpSaveRequest,
  defaultOmpSelection,
  draftFromProvider,
  isEditableOmpProvider,
  validateOmpDraft,
} from "@/pages/models/ompModels";
import type { OmpConfigRead, OmpProviderSummary } from "@/shared/features/omp";

const existing: OmpProviderSummary = {
  id: "team-gateway",
  baseUrl: "https://gateway.example/v1",
  api: null,
  auth: null,
  hasApiKey: true,
  hasCommandKey: true,
  models: [
    { id: "vendor/model-a", name: "Model A" },
    { id: "model-b", name: null },
  ],
};

const anthropic: OmpProviderSummary = {
  ...existing,
  id: "anthropic-proxy",
  api: "anthropic-messages",
};

const codex: OmpProviderSummary = {
  ...existing,
  id: "codex-proxy",
  api: "openai-codex-responses",
};

function read(providers: OmpProviderSummary[]): OmpConfigRead {
  return {
    profile: "work",
    modelsPath: "~/.omp/profiles/work/agent/models.yml",
    configPath: "~/.omp/profiles/work/agent/config.yml",
    modelsRevision: "models-rev",
    configRevision: "config-rev",
    providers,
    defaultModel: "team-gateway/vendor/model-a",
    modelsFileStatus: "ok",
    configFileStatus: "ok",
  };
}

describe("omp model draft", () => {
  it("defaults to the first editable provider and keeps unsupported protocols read-only", () => {
    expect(isEditableOmpProvider(anthropic)).toBe(false);
    expect(isEditableOmpProvider(codex)).toBe(false);
    expect(defaultOmpSelection(read([anthropic, existing]))).toBe(
      "existing:team-gateway",
    );
    expect(defaultOmpSelection(read([anthropic]))).toBe("new");
  });

  it("keeps the existing key reference and only resends current models", () => {
    const config = read([existing]);
    const draft = draftFromProvider(existing, config.defaultModel);
    expect(draft).toMatchObject({
      keySource: "keep",
      auth: "apiKey",
      defaultModelId: "vendor/model-a",
    });
    expect(validateOmpDraft(draft, existing, config)).toEqual({});

    const request = buildOmpSaveRequest(
      { ...draft, baseUrl: " https://new.example/v1 " },
      existing,
      config,
    );
    expect(request).toEqual({
      provider: {
        id: "team-gateway",
        baseUrl: "https://new.example/v1",
        api: "openai-completions",
        apiKey: null,
        auth: null,
        models: [
          { id: "vendor/model-a", name: "Model A" },
          { id: "model-b", name: null },
        ],
      },
      defaultModel: null,
      expectedModelsRevision: "models-rev",
      expectedConfigRevision: "config-rev",
    });
  });

  it("suggests the fyagent- prefix without requiring it", () => {
    const config = read([existing]);
    const base = draftFromProvider(null, null);
    expect(base.providerId).toBe(OMP_SUGGESTED_NEW_PROVIDER_ID);
    expect(OMP_SUGGESTED_NEW_PROVIDER_ID.startsWith("fyagent-")).toBe(true);
    const draft = {
      ...base,
      providerId: "lab",
      baseUrl: "http://127.0.0.1:8080/v1",
      api: "openai-responses" as const,
      envName: "LAB_KEY",
      models: [{ id: "org/model/v2", name: "Lab v2" }],
      setDefault: true,
      defaultModelId: "org/model/v2",
    };
    expect(validateOmpDraft(draft, null, config)).toEqual({});
    expect(buildOmpSaveRequest(draft, null, config)).toMatchObject({
      provider: {
        id: "lab",
        apiKey: "LAB_KEY",
        auth: "apiKey",
        models: [{ id: "org/model/v2", name: "Lab v2" }],
      },
      defaultModel: "lab/org/model/v2",
    });
  });

  it("rejects colliding identities, secrets as env names and empty model sets", () => {
    const config = read([existing]);
    const base = draftFromProvider(null, null);
    expect(
      validateOmpDraft(
        { ...base, providerId: "team-gateway", baseUrl: "https://x.example" },
        null,
        config,
      ),
    ).toMatchObject({ providerId: expect.any(String) });
    expect(
      validateOmpDraft(
        { ...base, providerId: "bad id!", baseUrl: "https://x.example" },
        null,
        config,
      ),
    ).toMatchObject({ providerId: expect.any(String) });
    expect(
      validateOmpDraft(
        {
          ...base,
          providerId: "fyagent-x",
          baseUrl: "https://user:pw@x.example",
          envName: "sk-live-123",
        },
        null,
        config,
      ),
    ).toEqual({
      baseUrl: expect.any(String),
      keyRef: expect.any(String),
      models: expect.any(String),
    });
    expect(
      validateOmpDraft(
        {
          ...base,
          providerId: "fyagent-x",
          baseUrl: "https://x.example",
          keySource: "command",
          commandRef: "op read key",
          models: [{ id: "/bad", name: null }],
          setDefault: true,
          defaultModelId: "missing",
        },
        null,
        config,
      ),
    ).toEqual({
      keyRef: expect.any(String),
      models: expect.any(String),
      defaultModel: expect.any(String),
    });
  });

  it("does not write auth or key for an OAuth-managed provider", () => {
    const oauth = { ...existing, auth: "oauth", hasApiKey: false };
    const config = read([oauth]);
    const draft = draftFromProvider(oauth, null);
    expect(validateOmpDraft(draft, oauth, config)).toEqual({});
    expect(buildOmpSaveRequest(draft, oauth, config).provider).toMatchObject({
      apiKey: null,
      auth: null,
    });
  });
});

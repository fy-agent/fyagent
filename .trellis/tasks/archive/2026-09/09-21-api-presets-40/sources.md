# Vendor verification for #40

Reviewed 2026-09-21. No authenticated vendor inference requests were sent.

- Alibaba [regional Base URLs](https://help.aliyun.com/en/model-studio/base-url)
  and [Coding Plan](https://help.aliyun.com/zh/model-studio/coding-plan) confirm
  Beijing pay-as-you-go compatible-mode/v1, Coding Plan coding.dashscope /v1
  and /apps/anthropic, and distinct plan keys/scopes.
- Alibaba [Codex integration](https://help.aliyun.com/zh/model-studio/codex)
  states Coding Plan supports Chat Completions rather than Responses and
  warns about newer direct Codex clients. The preset uses Chat and the form
  explains that compatibility condition without downgrading/installing clients
  or enabling a proxy.
- Tencent [TokenHub Codex integration](https://cloud.tencent.com/document/product/1823/133532),
  updated 2026-08-24, specifies tokenhub.tencentmaas.com/v1, hy3, Responses and
  Hy3 access scope for the API key. The legacy Hunyuan and Cloud SecretId
  products are not aliases for this preset.
- Ark [general API quick start](https://www.volcengine.com/docs/82379/1795150)
  confirms /api/v3 Responses with an Ark API key. Account-specific model ID is
  left for the user rather than assuming access to a sample model.
- Ark Coding Plan /api/coding/v3 Responses and ark-code-latest reuse the
  repository's reviewed 2026-07 Codex contract, cited to the
  [official Codex guide](https://www.volcengine.com/docs/82379/2556056).
  The provided source brief separately distinguishes the Anthropic /api/coding
  endpoint and general API quota. Direct re-fetch of the official guide failed
  in this session (web redirect/fetch error; curl returned only an app shell).
  This is reused reviewed source evidence, not a fresh successful retrieval or
  a real-service compatibility claim. Search surfaced inconsistent community
  articles; those were not used to override the reviewed Responses contract.

No prices, quotas, region availability, Key values or real model-response
claims are inferred from fixtures. Shared presets are product entry helpers;
users still need current account entitlement and a compatible target client.

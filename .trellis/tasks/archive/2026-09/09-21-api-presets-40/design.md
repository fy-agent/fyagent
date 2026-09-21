# Design

Extend the existing shared ProviderQuickSetupRequest with optional protocol:
anthropic/responses/chat. Claude accepts Anthropic, Codex Responses/Chat and
Grok Build Responses. Absent protocol keeps current defaults. No new Provider
writer, key store, management page or source-switch workflow.

Production presets live in one portable domain owner, consumed by a shared form
control in ProviderPanel. The control fills fields only and clears the old key
when changing provider/product. Key scope and tool restrictions are displayed
alongside the existing SecretInput. Native known-plan admission validates
endpoint/protocol/key mismatches and refuses generic restricted-plan probes.

Public summary gets an optional closed connection projection (baseUrl/modelId/
protocol), never a key/reference. A saved item can explicitly fill the existing
form for editing; Codex saved-source activation remains the existing Auth path.
No automatic effect hydrates over an unsaved draft. Save failures and restoration
remain owned by ProviderService and Change Plan.

Model probe request carries the same optional protocol; native request-body,
URL and headers use a single checked protocol selection. Old callers and
WorkBuddy/OpenCode retain their existing defaults. User-initiated provider
model discovery/probe refuses Coding Plan endpoints or dedicated plan keys.

Root owns Models save handlers and #73 final callback wiring. Changes here stay
in form state/fields and pure request helpers; one protocol input forwarding
field in requestSave is coordinated separately. Design uses the existing
restrained developer-tool form, semantic tokens and shared controls.

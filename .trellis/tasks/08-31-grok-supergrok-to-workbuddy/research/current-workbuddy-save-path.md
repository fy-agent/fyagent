# Research: current SuperGrok → WorkBuddy path

- **Query**: How WorkBuddy saves models today; whether SuperGrok / `xai_oauth` can bind without a second pasted key; what must not be copied into `models.json`.
- **Scope**: internal code + parent #42 / #106
- **Date**: 2026-08-31

## Findings

WorkBuddy is not a Provider app. Save goes through Change Plan operation `workbuddy_models_save`:

- Create: `create_workbuddy_save_plan` / `ports.changePlans.createWorkBuddySavePlan`
- Request: `SaveWorkBuddyModelsRequest` = `base_url` + `api_key` + model ids + revision / overwrite token
- On disk: `{trusted-home}/.workbuddy/models.json` (and backup). The live file stores `url` and `apiKey`.
- Public plan stays credential-free; the key lives in a process-private draft keyed by `planId`.

There is no WorkBuddy `xai_oauth` preset. Auth Center login does not change the WorkBuddy form by itself.

Copying the SuperGrok refresh token into `models.json` is out of scope: the token rotates, and it would leak a managed secret into another app's file.

`get_xai_oauth_models` can list models for a logged-in account. That can fill the WorkBuddy model id list without a second device-code login. WorkBuddy runtime still reads its own file; if that file only accepts a key, the save cannot honestly claim “OAuth bind” the way Codex does.

Qoder cannot take third-party models. TRAE cannot be written. Those stay out.

## Reuse owners

- Login: Auth Center / `xai_oauth`
- Model list (optional): `get_xai_oauth_models`
- Save: existing WorkBuddy Change Plan + `WorkBuddySavePlanWorkspace`

## What must stay out

- Codex upsert / Claude Provider bind
- Fourth Change Plan adapter
- Writing refresh tokens into `models.json`
- Turning WorkBuddy into `AppType`

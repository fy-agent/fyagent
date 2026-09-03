import { invoke } from "@tauri-apps/api/core";

import {
  assertManagedAuthAccountMutation,
  assertManagedAuthConnectionActionRequest,
  assertManagedAuthLoginMethod,
  assertManagedAuthRemovalMutation,
  assertManagedAuthSessionId,
  assertStartManagedAuthLoginRequest,
  parseManagedAuthLoginSession,
  parseManagedAuthMutationResult,
  parseManagedAuthOverview,
  parseManagedAuthRemovalPreview,
  type ManagedAuthPort,
} from "../../../features/managed-auth";

export function createManagedAuthPort(): ManagedAuthPort {
  return {
    getOverview: async () =>
      parseManagedAuthOverview(
        await invoke<unknown>("managed_auth_get_overview"),
      ),
    startLogin: async (request) =>
      parseManagedAuthLoginSession(
        await invoke<unknown>("managed_auth_start_login", {
          request: assertStartManagedAuthLoginRequest(request),
        }),
      ),
    getLoginSession: async (sessionId) =>
      parseManagedAuthLoginSession(
        await invoke<unknown>("managed_auth_get_login_session", {
          sessionId: assertManagedAuthSessionId(sessionId),
        }),
      ),
    cancelLogin: async (sessionId) =>
      parseManagedAuthLoginSession(
        await invoke<unknown>("managed_auth_cancel_login", {
          sessionId: assertManagedAuthSessionId(sessionId),
        }),
      ),
    reopenLogin: async (sessionId) =>
      parseManagedAuthLoginSession(
        await invoke<unknown>("managed_auth_reopen_login", {
          sessionId: assertManagedAuthSessionId(sessionId),
        }),
      ),
    switchLoginMethod: async (sessionId, method) =>
      parseManagedAuthLoginSession(
        await invoke<unknown>("managed_auth_switch_login_method", {
          sessionId: assertManagedAuthSessionId(sessionId),
          method: assertManagedAuthLoginMethod(method),
        }),
      ),
    setDefaultAccount: async (accountId, expectedRevision) =>
      parseManagedAuthMutationResult(
        await invoke<unknown>("managed_auth_set_default_account", {
          request: assertManagedAuthAccountMutation(
            accountId,
            expectedRevision,
          ),
        }),
      ),
    previewAccountRemoval: async (accountId, expectedRevision) =>
      parseManagedAuthRemovalPreview(
        await invoke<unknown>("managed_auth_preview_account_removal", {
          request: assertManagedAuthAccountMutation(
            accountId,
            expectedRevision,
          ),
        }),
      ),
    removeAccount: async (previewId, accountId, expectedRevision) =>
      parseManagedAuthMutationResult(
        await invoke<unknown>("managed_auth_remove_account", {
          request: assertManagedAuthRemovalMutation(
            previewId,
            accountId,
            expectedRevision,
          ),
        }),
      ),
    applyConnectionAction: async (request) =>
      parseManagedAuthMutationResult(
        await invoke<unknown>("managed_auth_apply_connection_action", {
          request: assertManagedAuthConnectionActionRequest(request),
        }),
      ),
  };
}

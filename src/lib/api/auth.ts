import { invoke } from "@tauri-apps/api/core";

export type ManagedAuthProvider =
  | "github_copilot"
  | "codex_oauth"
  | "xai_oauth";

export const LEGACY_AUTH_MUTATION_DISABLED = "legacy_auth_mutation_disabled";

export interface ManagedAuthAccount {
  id: string;
  provider: ManagedAuthProvider;
  login: string;
  avatar_url: string | null;
  authenticated_at: number;
  is_default: boolean;
  github_domain: string;
  requires_reauth: boolean;
  chatgpt_account_id?: string | null;
}

export interface ManagedAuthStatus {
  provider: ManagedAuthProvider;
  authenticated: boolean;
  default_account_id: string | null;
  migration_error?: string | null;
  accounts: ManagedAuthAccount[];
  native_projection_available?: boolean | null;
}

export interface ManagedAuthDeviceCodeResponse {
  provider: ManagedAuthProvider;
  device_code: string;
  user_code: string;
  verification_uri: string;
  expires_in: number;
  interval: number;
}

function denyLegacyAuthMutation(): never {
  throw new Error(LEGACY_AUTH_MUTATION_DISABLED);
}

export async function authStartLogin(
  _authProvider: ManagedAuthProvider,
  _githubDomain?: string,
): Promise<ManagedAuthDeviceCodeResponse> {
  denyLegacyAuthMutation();
}

export async function authPollForAccount(
  _authProvider: ManagedAuthProvider,
  _deviceCode: string,
  _githubDomain?: string,
): Promise<ManagedAuthAccount | null> {
  denyLegacyAuthMutation();
}

export async function authListAccounts(
  authProvider: ManagedAuthProvider,
): Promise<ManagedAuthAccount[]> {
  return invoke<ManagedAuthAccount[]>("auth_list_accounts", {
    authProvider,
  });
}

export async function authGetStatus(
  authProvider: ManagedAuthProvider,
): Promise<ManagedAuthStatus> {
  return invoke<ManagedAuthStatus>("auth_get_status", {
    authProvider,
  });
}

export async function authRemoveAccount(
  _authProvider: ManagedAuthProvider,
  _accountId: string,
): Promise<void> {
  denyLegacyAuthMutation();
}

export async function authSetDefaultAccount(
  _authProvider: ManagedAuthProvider,
  _accountId: string,
): Promise<void> {
  denyLegacyAuthMutation();
}

export async function authLogout(
  _authProvider: ManagedAuthProvider,
): Promise<void> {
  denyLegacyAuthMutation();
}

export async function authCancelLogin(
  _authProvider: ManagedAuthProvider,
  _deviceCode?: string | null,
): Promise<void> {
  denyLegacyAuthMutation();
}

export const authApi = {
  authStartLogin,
  authPollForAccount,
  authListAccounts,
  authGetStatus,
  authRemoveAccount,
  authSetDefaultAccount,
  authLogout,
  authCancelLogin,
};

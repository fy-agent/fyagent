import { useQuery } from "@tanstack/react-query";
import { authApi } from "@/lib/api";
import type { ManagedAuthProvider, ManagedAuthStatus } from "@/lib/api";

export function useManagedAuth(authProvider: ManagedAuthProvider) {
  const queryKey = ["managed-auth-status", authProvider];

  const {
    data: authStatus,
    isLoading: isLoadingStatus,
    refetch: refetchStatus,
  } = useQuery<ManagedAuthStatus>({
    queryKey,
    queryFn: () => authApi.authGetStatus(authProvider),
    staleTime: 30000,
    // A rejected xAI refresh token is persisted as `requires_reauth` by the
    // proxy hot path. Periodically refresh leftover picker status so an
    // already-open form stops offering an expired account as usable.
    refetchInterval: authProvider === "xai_oauth" ? 15_000 : false,
  });

  const accounts = authStatus?.accounts ?? [];

  return {
    authStatus,
    isLoadingStatus,
    accounts,
    hasAnyAccount: accounts.length > 0,
    isAuthenticated: authStatus?.authenticated ?? false,
    defaultAccountId: authStatus?.default_account_id ?? null,
    migrationError: authStatus?.migration_error ?? null,
    refetchStatus,
  };
}

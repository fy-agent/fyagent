import { useManagedAuth } from "./useManagedAuth";

export function useCopilotAuth() {
  return useManagedAuth("github_copilot");
}

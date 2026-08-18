import { createBrowserCredentialsPort } from "./browser";
import type { CredentialsPort } from "./types";

export type { CredentialsPort };

export function createCredentialsPort(): CredentialsPort {
  return createBrowserCredentialsPort();
}

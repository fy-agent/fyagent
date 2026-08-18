import type {
  BeginSecretCaptureRequest,
  CredentialsPort,
  CredentialsSnapshot,
} from "../../data/credentials";

function refuseInvoke(): never {
  throw new Error(
    "Tauri credentials adapter is not implemented and must not invoke Tauri",
  );
}

export function createTauriCredentialsPort(): CredentialsPort {
  return {
    source: "tauri-stub",
    async listWorkspace(): Promise<CredentialsSnapshot> {
      return refuseInvoke();
    },
    async beginCapture(_request: BeginSecretCaptureRequest): Promise<void> {
      return refuseInvoke();
    },
  };
}

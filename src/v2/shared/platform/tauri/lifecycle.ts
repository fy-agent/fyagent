import { emit } from "@tauri-apps/api/event";

const FRONTEND_DEEPLINK_READY_EVENT = "frontend-deeplink-ready";

export function emitFrontendDeeplinkReady(): Promise<void> {
  return emit(FRONTEND_DEEPLINK_READY_EVENT);
}

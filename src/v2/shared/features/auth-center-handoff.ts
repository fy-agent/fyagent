export const OPEN_AUTH_CENTER_EVENT = "fyagent:open-auth-center";

export function requestOpenAuthCenter(): void {
  window.dispatchEvent(new CustomEvent(OPEN_AUTH_CENTER_EVENT));
}

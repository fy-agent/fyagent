export type FirstUseGuideState = "pending" | "dismissed";

export function parseFirstUseGuideState(value: unknown): FirstUseGuideState {
  if (value === "pending" || value === "dismissed") return value;
  throw new Error("Invalid first-use guide state");
}

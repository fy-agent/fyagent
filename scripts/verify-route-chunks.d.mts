export type RouteChunkBudget = Readonly<{
  initialJavaScriptBytes: number;
  initialChunkBytes: number;
  initialCssBytes: number;
  routeChunkBytes: number;
}>;

export type RouteChunkRecord = Readonly<{
  route: string;
  file: string;
  bytes: number;
}>;

export type InitialChunkRecord = Readonly<{
  key: string;
  file: string;
  bytes: number;
}>;

export type RouteChunkVerification = Readonly<{
  initialJavaScriptBytes: number;
  initialCssBytes: number;
  initialChunks: InitialChunkRecord[];
  routeChunks: RouteChunkRecord[];
  deferredPortChunks: InitialChunkRecord[];
}>;

export const RENDERER_ROUTE_ENTRIES: readonly string[];
export const RENDERER_BOOTSTRAP_DEFERRED_PORT_ENTRIES: readonly string[];
export const RENDERER_DEFERRED_SHELL_ENTRIES: readonly string[];
export const RENDERER_NESTED_SUBSCRIPTION_PORT: Readonly<{
  importer: string;
  entry: string;
}>;
export const RENDERER_DEFERRED_PORT_ENTRIES: readonly string[];
export const RENDERER_BUILD_BUDGET: RouteChunkBudget;

export function verifyRouteChunks(options?: {
  distributionDirectory?: string;
  budget?: RouteChunkBudget;
}): Promise<RouteChunkVerification>;

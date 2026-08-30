export type V2RouteChunkBudget = Readonly<{
  initialJavaScriptBytes: number;
  initialChunkBytes: number;
  initialCssBytes: number;
  routeChunkBytes: number;
}>;

export type V2RouteChunkRecord = Readonly<{
  route: string;
  file: string;
  bytes: number;
}>;

export type V2InitialChunkRecord = Readonly<{
  key: string;
  file: string;
  bytes: number;
}>;

export type V2RouteChunkVerification = Readonly<{
  initialJavaScriptBytes: number;
  initialCssBytes: number;
  initialChunks: V2InitialChunkRecord[];
  routeChunks: V2RouteChunkRecord[];
}>;

export const V2_ROUTE_ENTRIES: readonly string[];
export const V2_BUILD_BUDGET: V2RouteChunkBudget;

export function verifyV2RouteChunks(options?: {
  distributionDirectory?: string;
  budget?: V2RouteChunkBudget;
}): Promise<V2RouteChunkVerification>;

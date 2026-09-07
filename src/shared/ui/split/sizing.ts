/** Pixel constraints belong to the adapter; CSS must not maintain a second layout. */
export interface SplitSizing {
  minWidths?: readonly number[];
  maxWidths?: ReadonlyArray<number | undefined>;
  defaultWidths?: ReadonlyArray<number | undefined>;
  /** The pane that absorbs group growth; defaults to the final pane. */
  flexiblePane?: number;
}

export const DETAIL_PANE_SIZING = {
  minWidths: [220, 360, 220],
  defaultWidths: [268, undefined, 280],
  maxWidths: [420, undefined, 360],
  flexiblePane: 1,
} as const satisfies SplitSizing;

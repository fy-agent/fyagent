import { useMediaQuery } from "../ui/useMediaQuery";

const WIDE_FEATURE_LAYOUT = "(min-width: 1181px)";

export function useWideFeatureLayout(): boolean {
  return useMediaQuery(WIDE_FEATURE_LAYOUT);
}

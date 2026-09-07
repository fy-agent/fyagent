import { useCallback, useState } from "react";
import { useSearchParams, type SetURLSearchParams } from "react-router-dom";

import { usePersistentVisibility } from "./PersistentSurface";

type SearchSnapshot = {
  visible: boolean;
  search: string;
  params: URLSearchParams;
};

/**
 * Route-owned search params that stay frozen while this tree is hidden.
 * Hidden keep-alive pages must not read or write the active route's query.
 */
export function usePersistentSearchParams(): {
  visible: boolean;
  searchParams: URLSearchParams;
  setSearchParams: SetURLSearchParams;
} {
  const visible = usePersistentVisibility();
  const [searchParams, setSearchParams] = useSearchParams();
  const liveSearch = searchParams.toString();
  const [snapshot, setSnapshot] = useState<SearchSnapshot>(() => ({
    visible,
    search: liveSearch,
    params: searchParams,
  }));

  if (visible && snapshot.search !== liveSearch) {
    setSnapshot({ visible: true, search: liveSearch, params: searchParams });
  } else if (visible !== snapshot.visible) {
    setSnapshot(
      visible
        ? { visible: true, search: liveSearch, params: searchParams }
        : { visible: false, search: snapshot.search, params: snapshot.params },
    );
  }

  const setVisibleSearchParams = useCallback<SetURLSearchParams>(
    (nextInit, navigateOpts) => {
      if (!visible) {
        return;
      }
      setSearchParams(nextInit, navigateOpts);
    },
    [visible, setSearchParams],
  );

  return {
    visible,
    searchParams: visible ? searchParams : snapshot.params,
    setSearchParams: setVisibleSearchParams,
  };
}

/**
 * Remembers the last explicit value while the surface is hidden, and when the
 * URL omits it after a keep-alive return (for example `/models` with no target).
 */
export function useStickyVisibleValue<T>(
  visible: boolean,
  explicit: T | null,
  fallback: T,
): T {
  const [value, setValue] = useState<T>(explicit ?? fallback);
  if (visible && explicit !== null && value !== explicit) {
    setValue(explicit);
  }
  if (visible && explicit !== null) {
    return explicit;
  }
  return value;
}

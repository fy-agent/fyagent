import { Outlet, useLocation } from "react-router-dom";
import { Suspense, useState, type ReactNode } from "react";

import {
  navigationItems,
  type NavigationItem,
} from "../shared/config/navigation";
import { PersistentSurface } from "../shared/ui/PersistentSurface";
import { primaryPages } from "./primaryPages";

function isPrimaryPath(pathname: string): pathname is NavigationItem["path"] {
  return navigationItems.some((item) => item.path === pathname);
}

function lazyPage(page: ReactNode): ReactNode {
  return (
    <Suspense
      fallback={
        <div className="fy-feature-route-loading" role="status">
          正在加载页面
        </div>
      }
    >
      {page}
    </Suspense>
  );
}

function withPath(
  current: ReadonlySet<NavigationItem["path"]>,
  pathname: NavigationItem["path"],
): ReadonlySet<NavigationItem["path"]> {
  if (current.has(pathname)) return current;
  const next = new Set(current);
  next.add(pathname);
  return next;
}

/**
 * Keeps visited primary routes mounted behind PersistentSurface.
 * New paths are registered with the React-allowed during-render setState
 * adjustment so the first paint already includes the destination page.
 */
export function PersistentPrimaryOutlet() {
  const { pathname } = useLocation();
  const [mountedPaths, setMountedPaths] = useState<
    ReadonlySet<NavigationItem["path"]>
  >(() => new Set([isPrimaryPath(pathname) ? pathname : "/agents"]));

  if (isPrimaryPath(pathname) && !mountedPaths.has(pathname)) {
    setMountedPaths((current) => withPath(current, pathname));
  }

  if (!isPrimaryPath(pathname)) {
    return <Outlet />;
  }

  return (
    <>
      {navigationItems
        .filter((item) => mountedPaths.has(item.path) || item.path === pathname)
        .map((item) => {
          const RoutedPage = primaryPages[item.id];
          return (
            <PersistentSurface key={item.id} active={item.path === pathname}>
              {lazyPage(<RoutedPage />)}
            </PersistentSurface>
          );
        })}
    </>
  );
}

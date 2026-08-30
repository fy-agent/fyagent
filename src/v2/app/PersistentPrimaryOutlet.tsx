import { Outlet } from "react-router-dom";

/**
 * Primary routes are active-route-only. Backend resources and long-running
 * jobs recover through their query/session owners when a route remounts; this
 * composition boundary deliberately owns no prior-route mount state.
 */
export function PersistentPrimaryOutlet() {
  return <Outlet />;
}

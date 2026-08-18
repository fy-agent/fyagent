import { GearSixIcon } from "@phosphor-icons/react/dist/csr/GearSix";
import { MagnifyingGlassIcon } from "@phosphor-icons/react/dist/csr/MagnifyingGlass";
import { UserIcon } from "@phosphor-icons/react/dist/csr/User";

import { IconButton, Tooltip } from "../../shared/ui/primitives";

const noop = () => undefined;

export function ToolCluster() {
  return (
    <div
      className="fy-tool-cluster"
      role="group"
      aria-label="工具"
      data-testid="tool-cluster"
    >
      <Tooltip label="Search">
        <IconButton aria-label="Search" data-testid="search" onClick={noop}>
          <MagnifyingGlassIcon size={19} weight="regular" aria-hidden />
        </IconButton>
      </Tooltip>

      <Tooltip label="Settings">
        <IconButton aria-label="Settings" data-testid="settings" onClick={noop}>
          <GearSixIcon size={19} weight="regular" aria-hidden />
        </IconButton>
      </Tooltip>

      <Tooltip label="Avatar">
        <IconButton
          className="fy-avatar-button"
          aria-label="Avatar"
          data-testid="avatar"
          onClick={noop}
        >
          <UserIcon size={18} weight="regular" aria-hidden />
        </IconButton>
      </Tooltip>
    </div>
  );
}

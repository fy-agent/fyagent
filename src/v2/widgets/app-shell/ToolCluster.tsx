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
      <Tooltip label="搜索">
        <IconButton aria-label="搜索" data-testid="search" onClick={noop}>
          <MagnifyingGlassIcon size={19} weight="regular" aria-hidden />
        </IconButton>
      </Tooltip>

      <Tooltip label="设置">
        <IconButton aria-label="设置" data-testid="settings" onClick={noop}>
          <GearSixIcon size={19} weight="regular" aria-hidden />
        </IconButton>
      </Tooltip>

      <Tooltip label="账户">
        <IconButton
          className="fy-avatar-button"
          aria-label="账户"
          data-testid="avatar"
          onClick={noop}
        >
          <UserIcon size={18} weight="regular" aria-hidden />
        </IconButton>
      </Tooltip>
    </div>
  );
}

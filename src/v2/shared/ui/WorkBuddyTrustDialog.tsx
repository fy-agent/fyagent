import { Button } from "./Button";
import { Dialog } from "./Dialog";

export function WorkBuddyTrustDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  return (
    <Dialog
      open={open}
      title="需要在 WorkBuddy 中信任 MCP"
      description="请到「连接器 → 自定义连接器」中信任该 MCP 后才能使用。"
      onOpenChange={onOpenChange}
      actions={
        <Button
          className="fy-control-button-primary"
          onClick={() => onOpenChange(false)}
        >
          知道了
        </Button>
      }
    >
      <p>WorkBuddy 官方限制第三方 MCP 必须在安装后手动信任授权才能正常使用。</p>
    </Dialog>
  );
}

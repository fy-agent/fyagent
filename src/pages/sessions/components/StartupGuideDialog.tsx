import { Dialog } from "../../../shared/ui/Dialog";
import { Button } from "../../../shared/ui/Button";
import type { DialogOriginRef } from "../../../shared/ui/dialogOrigin";
import { PROVIDER_LABELS } from "../../../shared/features/session-migration";

export interface StartupGuideDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  providerId: string;
  nativeSessionId?: string;
  originRef: DialogOriginRef | undefined;
}

const PROVIDER_STARTUP_GUIDES: Record<
  string,
  { channel: string; instructions: string }
> = {
  codex: {
    channel: "Codex CLI / Codex Desktop 会话通道",
    instructions:
      "Codex 0.154.0 采用隔离 CODEX_HOME 机制。通过本地协议恢复后，会话记录写入本地存储。可在目标终端中通过官方 resume 命令唤起，或在桌面端历史抽屉中查看。",
  },
  opencode: {
    channel: "OpenCode SQLite 本地存储",
    instructions:
      "OpenCode 会话数据位于其本地 SQLite 数据库中。写入恢复成功后，直接启动 OpenCode 即可在其会话侧栏中定位并继续对话。",
  },
  hermes: {
    channel: "Hermes CLI SQLite 会话账本",
    instructions:
      "Hermes 会话保存在本地 SQLite 数据库中。恢复完成后可在终端运行 hermes 并接续已有会话 ID。",
  },
  gemini: {
    channel: "Gemini CLI 本地存储",
    instructions:
      "Gemini CLI 采用本地 JSON 会话记录。恢复至目标设备配置目录后即可在官方 CLI 中续聊。",
  },
  claude: {
    channel: "Claude Code 本地配置与转录目录",
    instructions:
      "Claude Code 采用本地转录存储。恢复后在对应工作区拉起 claude 命令即可接续上下文。",
  },
  grokbuild: {
    channel: "Grok Build 本地配置",
    instructions:
      "恢复数据写入 Grok 本地存储。可在对应工程中启动 grok 客户端查看。",
  },
  openclaw: {
    channel: "OpenClaw 任务通道",
    instructions:
      "OpenClaw 会话以结构化任务形式落盘。写入目标工作区后通过 openclaw resume 接续。",
  },
};

export function StartupGuideDialog({
  open,
  onOpenChange,
  providerId,
  nativeSessionId,
  originRef,
}: StartupGuideDialogProps) {
  const guide = PROVIDER_STARTUP_GUIDES[providerId] ?? {
    channel: "对应软件原生通道",
    instructions: "请通过目标客户端官方入口查看会话列表并进行下一轮续聊。",
  };

  return (
    <Dialog
      open={open}
      originRef={originRef}
      onOpenChange={onOpenChange}
      size="comfortable"
      title="目标软件启动说明"
      description="各客户端启动参数与目标会话格式不同。系统不生成未经验证的可执行伪命令。"
      actions={
        <Button
          className="fy-control-button-primary"
          onClick={() => onOpenChange(false)}
        >
          了解
        </Button>
      }
    >
      <div className="fy-startup-guide-body">
        <div className="fy-guide-info-card">
          <div className="fy-guide-row">
            <span className="fy-guide-label">当前客户端：</span>
            <span className="fy-guide-value">
              {PROVIDER_LABELS[providerId] ?? providerId}
            </span>
          </div>

          <div className="fy-guide-row">
            <span className="fy-guide-label">原生接入方式：</span>
            <span className="fy-guide-value">{guide.channel}</span>
          </div>

          {nativeSessionId && (
            <div className="fy-guide-row">
              <span className="fy-guide-label">目标原生会话 ID：</span>
              <code className="fy-guide-code">{nativeSessionId}</code>
            </div>
          )}

          <div className="fy-guide-desc">{guide.instructions}</div>
        </div>

        <div className="fy-guide-security-notice">
          <strong>安全与边界：</strong>
          系统严格遵循最小权限原则，不接受来自迁移包的任意 shell
          字符串，亦不伪造未核验的自动化续聊。请在目标软件原生界面中验证下一轮对话。
        </div>
      </div>
    </Dialog>
  );
}

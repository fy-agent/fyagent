import { useState } from "react";
import type { McpServer } from "../../shared/features/mcp";
import { ExternalLinkButton } from "../../shared/features/controls/ExternalLinkButton";
import { Button } from "../../shared/ui/Button";
import { Dialog } from "../../shared/ui/Dialog";
import type { DialogOriginRef } from "../../shared/ui/dialogOrigin";
import { InlineNotice } from "../../shared/ui/primitives";
import {
  buildMcpInspectorConfig,
  MCP_INSPECTOR_INITIALIZE_COMMAND,
  MCP_INSPECTOR_TOOLS_COMMAND,
} from "./inspector";

export function InspectorDialog({
  server,
  originRef,
  onClose,
}: {
  server: McpServer;
  originRef: DialogOriginRef;
  onClose: () => void;
}) {
  const [message, setMessage] = useState<string | null>(null);
  const copy = async () => {
    try {
      const config = buildMcpInspectorConfig(server.server);
      await navigator.clipboard.writeText(config);
      setMessage("配置已复制，请保存为本机文件 mcp-inspector.json");
    } catch {
      setMessage("无法复制配置，请检查命令或地址，并允许访问剪贴板后重试");
    }
  };
  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
      originRef={originRef}
      title="本地协议诊断"
      description="源配置可能包含凭据，复制时已遮住；实际诊断前请仅在本机补入凭据。FyAgent 不会执行服务器命令或改变分配状态。"
      actions={
        <>
          <Button onClick={onClose}>关闭</Button>
          <Button onClick={() => void copy()}>复制 Inspector 配置</Button>
        </>
      }
    >
      <p>将配置保存为 mcp-inspector.json 后，在本机终端按顺序运行：</p>
      <ol>
        <li>
          先检查启动、连接和初始化：
          <pre className="fy-feature-code fy-mcp-inspector-command">
            {MCP_INSPECTOR_INITIALIZE_COMMAND}
          </pre>
        </li>
        <li>
          初始化成功后检查工具发现：
          <pre className="fy-feature-code fy-mcp-inspector-command">
            {MCP_INSPECTOR_TOOLS_COMMAND}
          </pre>
        </li>
      </ol>
      <p>
        两步均设置 15 秒超时，可按 Ctrl+C
        取消。仅列出工具，不调用工具。启动失败请检查命令与工作目录；连接失败请检查地址；授权失败请检查凭据；初始化成功后仍无法列出工具，请检查服务器日志。
      </p>
      <ExternalLinkButton url="https://github.com/modelcontextprotocol/inspector/blob/main/clients/cli/README.md">
        Inspector 使用说明
      </ExternalLinkButton>
      {message && <InlineNotice>{message}</InlineNotice>}
    </Dialog>
  );
}

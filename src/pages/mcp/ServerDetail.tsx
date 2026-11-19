import { CaretDownIcon } from "@phosphor-icons/react/dist/csr/CaretDown";
import { useState } from "react";

import { mcpInstallDirectory } from "../../shared/features/helpers";
import { redactMcpArgs, redactMcpUrl } from "../../shared/features/mcpSecurity";
import { MCP_IMPORT_SOURCES } from "../../shared/features/mcp";
import {
  MCP_TARGETS,
  type McpServer,
  type McpTargetId,
} from "../../shared/features/types";
import { Button } from "../../shared/ui/Button";
import {
  Collapsible,
  CollapsibleCaret,
  CollapsibleContent,
  CollapsibleTrigger,
} from "../../shared/ui/Collapsible";
import type { DialogOriginRef } from "../../shared/ui/dialogOrigin";
import { Badge } from "../../shared/ui/primitives";
import { AssignmentPanel } from "../../shared/ui/AssignmentPanel";
import { CopyablePath } from "../../shared/features/controls/CopyablePath";
import { ExternalLinkButton } from "../../shared/features/controls/ExternalLinkButton";
import { findCatalogItem, MCP_PROVENANCE_LABEL } from "./catalog";
import { transportOf } from "./constants";

export function ServerDetail({
  originRef,
  server,
  busy,
  onToggle,
  onEdit,
  onDelete,
  onInspect,
  showAssignment,
}: {
  originRef?: DialogOriginRef;
  server: McpServer;
  busy: boolean;
  onToggle: (app: McpTargetId, enabled: boolean) => void;
  onEdit: () => void;
  onDelete: () => void;
  onInspect: () => void;
  showAssignment: boolean;
}) {
  const [installationOpen, setInstallationOpen] = useState(false);
  const spec = server.server;
  const transport = transportOf(server);
  const catalogItem = findCatalogItem(server.id);
  const sourceLabel = server.sources?.length
    ? server.sources
        .map(
          (id) => MCP_IMPORT_SOURCES.find((source) => source.id === id)?.label,
        )
        .join("、")
    : "未记录导入来源";
  const assignedCount = MCP_IMPORT_SOURCES.filter(
    (source) => server.apps[source.id],
  ).length;
  const installDirectory = mcpInstallDirectory(spec);
  const description = server.description?.trim() || catalogItem?.description;
  const homepage = server.homepage || catalogItem?.homepage;
  const docs = server.docs || catalogItem?.docs;

  return (
    <section
      className="fy-feature-panel fy-feature-detail fy-feature-detail-scroll"
      aria-label="MCP 详情"
    >
      <div className="fy-feature-detail-header">
        <div className="fy-feature-detail-title">
          <h2>{server.name}</h2>
          <Badge tone="accent">{transport}</Badge>
          <Badge tone="neutral">
            {server.sources && server.sources.length > 1
              ? `${server.sources.length} 个导入来源`
              : sourceLabel}
          </Badge>
        </div>
        {description && <p className="fy-feature-intro">{description}</p>}
        <p className="fy-feature-description">
          {assignedCount ? `已分配 ${assignedCount} 个目标` : "尚未分配"}
        </p>
        <div className="fy-feature-actions">
          <Button dialogOriginRef={originRef} onClick={onEdit} disabled={busy}>
            编辑
          </Button>
          <Button
            dialogOriginRef={originRef}
            onClick={onInspect}
            disabled={busy}
          >
            本地协议诊断
          </Button>
          <Button
            className="fy-control-button-danger"
            onClick={onDelete}
            dialogOriginRef={originRef}
            disabled={busy}
          >
            删除
          </Button>
        </div>
      </div>
      <div className="fy-feature-info-grid">
        <Collapsible
          open={installationOpen}
          onOpenChange={setInstallationOpen}
          asChild
        >
          <section className="fy-feature-info-card" aria-label="安装信息">
            <h3>
              <CollapsibleTrigger asChild>
                <Button>
                  安装信息
                  <CollapsibleCaret open={installationOpen}>
                    <CaretDownIcon size={16} />
                  </CollapsibleCaret>
                </Button>
              </CollapsibleTrigger>
            </h3>
            <CollapsibleContent open={installationOpen}>
              <dl className="fy-feature-definition">
                <dt>导入来源</dt>
                <dd>{sourceLabel}</dd>
                {catalogItem && (
                  <>
                    <dt>发布方</dt>
                    <dd>{catalogItem.publisher}</dd>
                    <dt>目录收录依据</dt>
                    <dd>{MCP_PROVENANCE_LABEL[catalogItem.provenance]}</dd>
                  </>
                )}
                <dt>ID</dt>
                <dd>
                  <code className="fy-feature-code">{server.id}</code>
                </dd>
                {installDirectory && (
                  <>
                    <dt>安装目录</dt>
                    <dd>
                      <CopyablePath
                        revealValue={false}
                        value={installDirectory}
                      />
                    </dd>
                  </>
                )}
                {spec.command && (
                  <>
                    <dt>命令</dt>
                    <dd>
                      <code className="fy-feature-code">{spec.command}</code>
                    </dd>
                  </>
                )}
                {spec.args && spec.args.length > 0 && (
                  <>
                    <dt>参数</dt>
                    <dd>
                      {redactMcpArgs(spec.args).map((argument, index) => (
                        <code
                          className="fy-feature-code"
                          key={`${argument}-${index}`}
                        >
                          {argument}
                        </code>
                      ))}
                    </dd>
                  </>
                )}
                {spec.cwd && spec.cwd.trim() !== installDirectory && (
                  <>
                    <dt>工作目录</dt>
                    <dd>
                      <code className="fy-feature-code">{spec.cwd}</code>
                    </dd>
                  </>
                )}
                {spec.url && (
                  <>
                    <dt>URL</dt>
                    <dd>
                      <code className="fy-feature-code">
                        {redactMcpUrl(spec.url)}
                      </code>
                    </dd>
                  </>
                )}
                {spec.env && (
                  <>
                    <dt>环境变量</dt>
                    <dd>{Object.keys(spec.env).length} 项（仅在编辑时显示）</dd>
                  </>
                )}
                {spec.headers && (
                  <>
                    <dt>请求头</dt>
                    <dd>
                      {Object.keys(spec.headers).length} 项（仅在编辑时显示）
                    </dd>
                  </>
                )}
              </dl>
              {(homepage || docs) && (
                <div className="fy-feature-actions">
                  {homepage && (
                    <ExternalLinkButton url={homepage}>主页</ExternalLinkButton>
                  )}
                  {docs && (
                    <ExternalLinkButton url={docs}>说明</ExternalLinkButton>
                  )}
                </div>
              )}
            </CollapsibleContent>
          </section>
        </Collapsible>
      </div>
      {showAssignment && (
        <div className="fy-feature-inline-assignment">
          <AssignmentPanel
            dialogOriginRef={originRef}
            apps={server.apps}
            disabled={busy}
            labelSuffix="MCP 分配"
            onToggle={onToggle}
            targets={MCP_TARGETS}
          />
        </div>
      )}
    </section>
  );
}

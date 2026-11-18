import { useEffect, useRef, useState, type MouseEvent } from "react";

import {
  errorMessage,
  overlayKnownMcpFields,
  parseAdvancedServerJson,
  parseKeyValueLines,
  UserFacingError,
} from "../../shared/features/helpers";
import { mcpPresets } from "../../shared/features/presets";
import {
  createMcpAssignments,
  MCP_TARGETS,
  type McpServer,
  type McpServerSpec,
  type McpTargetId,
} from "../../shared/features/types";
import { Button } from "../../shared/ui/Button";
import type { DialogOriginRef } from "../../shared/ui/dialogOrigin";
import { Dialog } from "../../shared/ui/Dialog";
import { InlineNotice, Input } from "../../shared/ui/primitives";
import { AssignmentPanel } from "../../shared/ui/AssignmentPanel";
import { FeatureTabPanel, FeatureTabs } from "../../shared/ui/FeatureTabs";
import { DEFAULT_NEW_APPS } from "./constants";

type Mode = "quick" | "advanced";

export function McpEditor({
  creationTarget,
  originRef,
  initial,
  existingIds,
  busy,
  onClose,
  onSave,
}: {
  originRef?: DialogOriginRef;
  creationTarget?: McpTargetId;
  initial: McpServer | null;
  existingIds: Set<string>;
  busy: boolean;
  onClose: () => void;
  onSave: (
    server: McpServer,
    event: MouseEvent<HTMLButtonElement>,
  ) => Promise<string | null>;
}) {
  const spec = initial?.server ?? {};
  const [id, setId] = useState(initial?.id ?? "");
  const [name, setName] = useState(initial?.name ?? "");
  const [description, setDescription] = useState(initial?.description ?? "");
  const [tags, setTags] = useState((initial?.tags ?? []).join(", "));
  const [homepage, setHomepage] = useState(initial?.homepage ?? "");
  const [docs, setDocs] = useState(initial?.docs ?? "");
  const [transport, setTransport] = useState<"stdio" | "http" | "sse">(
    spec.type === "http" || spec.type === "sse" ? spec.type : "stdio",
  );
  const [command, setCommand] = useState(spec.command ?? "");
  const [args, setArgs] = useState((spec.args ?? []).join("\n"));
  const [cwd, setCwd] = useState(spec.cwd ?? "");
  const [url, setUrl] = useState(spec.url ?? "");
  const [env, setEnv] = useState(
    Object.entries(spec.env ?? {})
      .map(([key, value]) => `${key}=${value}`)
      .join("\n"),
  );
  const [headers, setHeaders] = useState(
    Object.entries(spec.headers ?? {})
      .map(([key, value]) => `${key}: ${value}`)
      .join("\n"),
  );
  const [apps, setApps] = useState(() =>
    initial
      ? { ...initial.apps }
      : createMcpAssignments(
          creationTarget ? [creationTarget] : DEFAULT_NEW_APPS,
        ),
  );
  const [mode, setMode] = useState<Mode>("quick");
  const [advanced, setAdvanced] = useState(JSON.stringify(spec, null, 2));
  const [errors, setErrors] = useState<string[]>([]);
  const formRef = useRef<HTMLDivElement>(null);
  const errorField = useRef("name");
  useEffect(() => {
    if (!errors.length) return;
    const field = formRef.current?.querySelector<HTMLElement>(
      `[data-mcp-field="${errorField.current}"]`,
    );
    field?.focus();
    field?.scrollIntoView?.({ block: "nearest" });
  }, [errors]);
  const [preset, setPreset] = useState("custom");
  const original = useRef<McpServer | null>(
    initial ? structuredClone(initial) : null,
  );
  const draft = useRef<McpServerSpec>(structuredClone(spec));
  const applyPreset = (presetId: string) => {
    setPreset(presetId);
    if (presetId === "custom") return;
    const value = mcpPresets.find((item) => item.id === presetId);
    if (!value) return;
    setId(value.id);
    setName(value.name);
    setTags((value.tags ?? []).join(", "));
    setHomepage(value.homepage ?? "");
    setDocs(value.docs ?? "");
    setTransport(
      value.server.type === "http" || value.server.type === "sse"
        ? value.server.type
        : "stdio",
    );
    setCommand(value.server.command ?? "");
    setArgs((value.server.args ?? []).join("\n"));
    setCwd(value.server.cwd ?? "");
    setUrl(value.server.url ?? "");
    setEnv("");
    setHeaders("");
    draft.current = structuredClone(value.server);
    setAdvanced(JSON.stringify(value.server, null, 2));
  };
  const applySpecToQuickForm = (value: McpServerSpec) => {
    setTransport(
      value.type === "http" || value.type === "sse" ? value.type : "stdio",
    );
    setCommand(value.command ?? "");
    setArgs((value.args ?? []).join("\n"));
    setCwd(value.cwd ?? "");
    setUrl(value.url ?? "");
    setEnv(
      Object.entries(value.env ?? {})
        .map(([key, item]) => `${key}=${item}`)
        .join("\n"),
    );
    setHeaders(
      Object.entries(value.headers ?? {})
        .map(([key, item]) => `${key}: ${item}`)
        .join("\n"),
    );
  };
  const quickSpec = (): McpServerSpec => {
    const envResult = parseKeyValueLines(env, "env");
    const headersResult = parseKeyValueLines(headers, "headers");
    if (envResult.errors.length || headersResult.errors.length)
      throw new UserFacingError(
        [
          ...envResult.errors.map((item) => `环境变量：${item}`),
          ...headersResult.errors.map((item) => `请求头：${item}`),
        ].join("；"),
      );
    if (transport === "stdio") {
      if (!command.trim()) throw new UserFacingError("请填写启动命令。");
      return {
        type: "stdio",
        command: command.trim(),
        ...(args.trim() ? { args: args.split(/\r?\n/).filter(Boolean) } : {}),
        ...(cwd.trim() ? { cwd: cwd.trim() } : {}),
        ...(Object.keys(envResult.value).length
          ? { env: envResult.value }
          : {}),
      };
    }
    if (!url.trim()) throw new UserFacingError("请填写连接地址。");
    try {
      new URL(url.trim());
    } catch {
      throw new UserFacingError("连接地址格式无效。");
    }
    return {
      type: transport,
      url: url.trim(),
      ...(Object.keys(headersResult.value).length
        ? { headers: headersResult.value }
        : {}),
    };
  };
  const switchMode = (next: Mode) => {
    try {
      if (next === "advanced") {
        draft.current = overlayKnownMcpFields(draft.current, quickSpec());
        setAdvanced(JSON.stringify(draft.current, null, 2));
      } else {
        draft.current = parseAdvancedServerJson(advanced);
        applySpecToQuickForm(draft.current);
      }
      setMode(next);
      setErrors([]);
    } catch (error) {
      errorField.current =
        mode === "advanced"
          ? "advanced"
          : transport === "stdio"
            ? "command"
            : "url";
      setErrors([errorMessage(error)]);
    }
  };
  const submit = (event: MouseEvent<HTMLButtonElement>) => {
    const nextErrors: string[] = [];
    const trimmedId = id.trim();
    if (!trimmedId) nextErrors.push("ID 为必填项");
    if (!initial && existingIds.has(trimmedId)) nextErrors.push("ID 已存在");
    if (!name.trim()) nextErrors.push("名称为必填项");
    let spec: McpServerSpec | null = null;
    try {
      spec =
        mode === "advanced"
          ? parseAdvancedServerJson(advanced)
          : overlayKnownMcpFields(draft.current, quickSpec());
    } catch (error) {
      nextErrors.push(errorMessage(error));
    }
    if (nextErrors.length || !spec) {
      errorField.current =
        !trimmedId || (!initial && existingIds.has(trimmedId))
          ? "id"
          : !name.trim()
            ? "name"
            : mode === "advanced"
              ? "advanced"
              : transport === "stdio"
                ? "command"
                : "url";
      setErrors(nextErrors);
      return;
    }
    const base = original.current ?? {};
    setErrors([]);
    void onSave(
      {
        ...base,
        id: initial?.id ?? trimmedId,
        name: name.trim(),
        server: spec,
        apps: { ...(initial?.apps ?? {}), ...apps },
        description: description.trim() || undefined,
        tags: tags
          .split(",")
          .map((item) => item.trim())
          .filter(Boolean),
        homepage: homepage.trim() || undefined,
        docs: docs.trim() || undefined,
      } as McpServer,
      event,
    ).then((failure) => {
      if (!failure) return;
      errorField.current = "name";
      setErrors([failure]);
    });
  };
  return (
    <Dialog
      originRef={originRef}
      open
      onOpenChange={(next) => !next && !busy && onClose()}
      title={initial ? `编辑 ${initial.name}` : "添加 MCP"}
      size="wide"
      actions={
        <div className="fy-mcp-editor-actions">
          {errors.length > 0 && (
            <div className="fy-mcp-editor-error" tabIndex={0}>
              <InlineNotice tone="error">{errors.join("；")}</InlineNotice>
            </div>
          )}
          <div className="fy-mcp-editor-buttons">
            <Button onClick={onClose} disabled={busy}>
              取消
            </Button>
            <Button
              className="fy-control-button-primary"
              onClick={submit}
              disabled={busy}
            >
              {busy ? "保存中…" : "保存"}
            </Button>
          </div>
        </div>
      }
    >
      <div ref={formRef} className="fy-feature-form-grid">
        {!initial && (
          <label className="fy-control-field">
            模板
            <select
              className="fy-control-select"
              value={preset}
              onChange={(event) => applyPreset(event.target.value)}
            >
              <option value="custom">自定义</option>
              {mcpPresets.map((item) => (
                <option key={item.id} value={item.id}>
                  {item.name}
                </option>
              ))}
            </select>
          </label>
        )}
        <label className="fy-control-field">
          ID
          <Input
            data-mcp-field="id"
            value={id}
            onChange={(event) => setId(event.target.value)}
            disabled={Boolean(initial)}
          />
        </label>
        <label className="fy-control-field">
          名称
          <Input
            data-mcp-field="name"
            value={name}
            onChange={(event) => setName(event.target.value)}
          />
        </label>
        <label className="fy-control-field">
          描述
          <Input
            value={description}
            onChange={(event) => setDescription(event.target.value)}
          />
        </label>
        <label className="fy-control-field">
          标签（逗号分隔）
          <Input
            value={tags}
            onChange={(event) => setTags(event.target.value)}
          />
        </label>
        <label className="fy-control-field">
          主页
          <Input
            value={homepage}
            onChange={(event) => setHomepage(event.target.value)}
          />
        </label>
        <label className="fy-control-field">
          文档
          <Input
            value={docs}
            onChange={(event) => setDocs(event.target.value)}
          />
        </label>
        <FeatureTabs
          id="mcp-editor-mode-tabs"
          className="fy-feature-form-span"
          label="编辑模式"
          value={mode}
          onChange={switchMode}
          options={[
            { id: "quick", label: "快速配置" },
            { id: "advanced", label: "JSON 编辑" },
          ]}
        />
        <FeatureTabPanel
          tabsId="mcp-editor-mode-tabs"
          value="quick"
          layout="flow"
          active={mode === "quick"}
          unmountOnExit
          className="fy-feature-form-tab-panel"
        >
          <>
            <label className="fy-control-field">
              传输类型
              <select
                className="fy-control-select"
                value={transport}
                onChange={(event) =>
                  setTransport(event.target.value as typeof transport)
                }
              >
                <option value="stdio">stdio</option>
                <option value="http">http</option>
                <option value="sse">sse</option>
              </select>
            </label>
            {transport === "stdio" ? (
              <>
                <label className="fy-control-field">
                  命令
                  <Input
                    data-mcp-field="command"
                    value={command}
                    onChange={(event) => setCommand(event.target.value)}
                  />
                </label>
                <label className="fy-control-field fy-feature-form-span">
                  参数（每行一个）
                  <textarea
                    className="fy-control-textarea"
                    rows={4}
                    value={args}
                    onChange={(event) => setArgs(event.target.value)}
                  />
                </label>
                <label className="fy-control-field">
                  工作目录
                  <Input
                    value={cwd}
                    onChange={(event) => setCwd(event.target.value)}
                  />
                </label>
                <label className="fy-control-field fy-feature-form-span">
                  环境变量（KEY=VALUE）
                  <textarea
                    className="fy-control-textarea"
                    rows={4}
                    value={env}
                    onChange={(event) => setEnv(event.target.value)}
                  />
                </label>
              </>
            ) : (
              <>
                <label className="fy-control-field fy-feature-form-span">
                  URL
                  <Input
                    data-mcp-field="url"
                    value={url}
                    onChange={(event) => setUrl(event.target.value)}
                  />
                </label>
                <label className="fy-control-field fy-feature-form-span">
                  请求头（Name: Value 或 Name=Value）
                  <textarea
                    className="fy-control-textarea"
                    rows={4}
                    value={headers}
                    onChange={(event) => setHeaders(event.target.value)}
                  />
                </label>
              </>
            )}
          </>
        </FeatureTabPanel>
        <FeatureTabPanel
          tabsId="mcp-editor-mode-tabs"
          value="advanced"
          layout="flow"
          active={mode === "advanced"}
          unmountOnExit
          className="fy-feature-form-tab-panel"
        >
          <label className="fy-control-field fy-feature-form-span">
            单个服务配置（JSON）
            <textarea
              className="fy-control-textarea"
              rows={14}
              data-mcp-field="advanced"
              value={advanced}
              onChange={(event) => setAdvanced(event.target.value)}
              spellCheck={false}
            />
          </label>
        </FeatureTabPanel>
        <div className="fy-feature-form-span">
          <p className="fy-feature-description">
            {MCP_TARGETS.some((target) => apps[target.id])
              ? "保存后仅分配给所选 Agent。"
              : "仅保存到 MCP 库，尚未分配给任何 Agent。"}
          </p>
          <AssignmentPanel
            apps={apps}
            disabled={busy}
            labelSuffix="MCP 分配"
            onToggle={(app, enabled) =>
              setApps((current) => ({ ...current, [app]: enabled }))
            }
            targets={MCP_TARGETS}
          />
        </div>
      </div>
    </Dialog>
  );
}

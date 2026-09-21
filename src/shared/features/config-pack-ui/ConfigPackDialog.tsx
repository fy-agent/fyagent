import { useEffect, useId, useRef, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  CONFIG_PACK_MAX_ENTRIES,
  type ExportPreview,
  type ImportChoice,
  type ImportPreview,
  type ImportResult,
  type PortableProvider,
} from "../../../domain/config-pack";
import { useFeatures } from "../provider";
import { safePackError, type ConfigPackFormFill } from "../config-pack";
import { featureKeys } from "../queries";
import { Button } from "../../ui/Button";
import { Dialog } from "../../ui/Dialog";
import type { DialogOriginRef } from "../../ui/dialogOrigin";
import { FeatureTabPanel, FeatureTabs } from "../../ui/FeatureTabs";
import { Checkbox, InlineNotice, Input, Spinner } from "../../ui/primitives";
import "./config-pack.css";

const appNames = { claude: "Claude Code", codex: "Codex" };
const actionNames = {
  add: "新增",
  skip: "跳过",
  rename: "另存为",
  overwrite: "覆盖未启用草稿",
};

function ProviderFields({ provider }: { provider: PortableProvider }) {
  return (
    <dl className="fy-config-pack-fields">
      <dt>应用</dt>
      <dd>{appNames[provider.app]}</dd>
      <dt>名称</dt>
      <dd>{provider.name}</dd>
      <dt>服务地址</dt>
      <dd>{provider.endpoint}</dd>
      <dt>模型</dt>
      <dd>{provider.model}</dd>
      {provider.wireApi && (
        <>
          <dt>接口</dt>
          <dd>
            {provider.wireApi === "responses"
              ? "Responses"
              : "Chat Completions"}
          </dd>
        </>
      )}
    </dl>
  );
}

export function ConfigPackDialog({
  open,
  originRef,
  onClose,
  onFillModelForm,
}: {
  open: boolean;
  originRef: DialogOriginRef;
  onClose: () => void;
  onFillModelForm?: ConfigPackFormFill;
}) {
  const { ports } = useFeatures();
  const port = ports.configPack;
  const client = useQueryClient();
  const tabsId = useId();
  const [mode, setMode] = useState<"export" | "import">("export");
  const [selection, setSelection] = useState<string[]>([]);
  const [text, setText] = useState("");
  const [choices, setChoices] = useState<ImportChoice[]>([]);
  const [preview, setPreview] = useState<ImportPreview | null>(null);
  const [previewCurrent, setPreviewCurrent] = useState(false);
  const [exported, setExported] = useState<ExportPreview | null>(null);
  const [result, setResult] = useState<ImportResult | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const lock = useRef(false);
  const alive = useRef(true);
  const ownedPreviews = useRef(new Set<string>());
  const candidates = useQuery({
    queryKey: featureKeys.configPackCandidates,
    queryFn: () => port.list(),
    enabled: open && mode === "export",
    staleTime: 0,
    retry: false,
  });
  useEffect(() => {
    alive.current = true;
    const owned = ownedPreviews.current;
    return () => {
      alive.current = false;
      for (const id of owned) void port.cancel(id).catch(() => {});
      owned.clear();
    };
  }, [port]);

  const own = (id: string) => {
    if (!alive.current) {
      void port.cancel(id).catch(() => {});
      return false;
    }
    ownedPreviews.current.add(id);
    return true;
  };
  const forget = (id: string | undefined) => {
    if (!id) return;
    ownedPreviews.current.delete(id);
    void port.cancel(id).catch(() => {});
  };
  const run = async (fn: () => Promise<void>) => {
    if (lock.current || !open || !alive.current) return;
    lock.current = true;
    setBusy(true);
    setError("");
    setNotice("");
    try {
      await fn();
    } catch (e) {
      if (alive.current) setError(safePackError(e).message);
    } finally {
      lock.current = false;
      if (alive.current) setBusy(false);
    }
  };
  const resetPreview = () => {
    forget(preview?.previewId);
    setPreviewCurrent(false);
    setResult(null);
  };
  const readText = (value: string) => {
    resetPreview();
    setPreview(null);
    setText(value);
    setChoices([]);
  };
  const updateChoice = (index: number, choice: ImportChoice) => {
    resetPreview();
    setChoices((current) =>
      current.map((item, i) => (i === index ? choice : item)),
    );
  };
  const prepareImport = () =>
    run(async () => {
      forget(preview?.previewId);
      setPreviewCurrent(false);
      const next = await port.previewImport(text, choices);
      if (!own(next.previewId)) return;
      setPreview(next);
      setPreviewCurrent(true);
      setResult(null);
      setChoices(
        next.entries.map((entry) => ({
          action: entry.action,
          name: entry.action === "rename" ? entry.provider.name : null,
        })),
      );
    });
  const saveImport = () =>
    run(async () => {
      if (!preview || !previewCurrent) return;
      setPreviewCurrent(false);
      const saved = await port.apply(preview);
      ownedPreviews.current.delete(preview.previewId);
      await Promise.all([
        client.invalidateQueries({
          queryKey: featureKeys.configPackCandidates,
        }),
        client.invalidateQueries({
          queryKey: featureKeys.providerSummary("claude"),
        }),
        client.invalidateQueries({
          queryKey: featureKeys.providerSummary("codex"),
        }),
      ]);
      if (alive.current) setResult(saved);
    });
  const fillModelForm = (provider: PortableProvider) =>
    run(async () => {
      if (!onFillModelForm) return;
      onFillModelForm({ ...provider });
      onClose();
    });

  return (
    <Dialog
      open={open}
      originRef={originRef}
      onOpenChange={(next) => {
        if (!next && !lock.current) onClose();
      }}
      title="迁移连接配置"
      size="wide"
      presentationKey={mode}
      description="本版支持已保存的 Claude Code 和 Codex 连接。API Key、账户绑定和本机专属设置不会导出；导入只保存未启用草稿。"
      actions={
        <>
          <Button disabled={busy} onClick={onClose}>
            关闭
          </Button>
          {mode === "import" && !result && (
            <>
              <Button
                disabled={busy || !text.trim()}
                onClick={() => void prepareImport()}
              >
                {preview ? "更新预览" : "预览导入"}
              </Button>
              <Button
                className="fy-control-button-primary"
                disabled={
                  busy ||
                  !previewCurrent ||
                  !preview?.entries.some((e) => e.action !== "skip")
                }
                onClick={() => void saveImport()}
              >
                确认保存
              </Button>
            </>
          )}
        </>
      }
    >
      <div className="fy-config-pack">
        <FeatureTabs
          id={tabsId}
          label="配置迁移方式"
          value={mode}
          options={[
            { id: "export", label: "导出" },
            { id: "import", label: "导入" },
          ]}
          onChange={(next) => {
            if (!busy) {
              setMode(next);
              setError("");
              setNotice("");
            }
          }}
        />
        {busy && <Spinner label="正在处理配置" />}
        {error && <InlineNotice tone="error">{error}</InlineNotice>}
        {notice && <InlineNotice>{notice}</InlineNotice>}
        <FeatureTabPanel
          tabsId={tabsId}
          value="export"
          active={mode === "export"}
          layout="flow"
        >
          {candidates.isPending && <Spinner label="正在读取连接" />}
          {candidates.isError && (
            <InlineNotice tone="error">
              无法读取连接。
              <Button disabled={busy} onClick={() => void candidates.refetch()}>
                重试
              </Button>
            </InlineNotice>
          )}
          {candidates.data && (
            <>
              {candidates.data.entries.length === 0 && (
                <p>暂无可导出的连接。请先在模型管理中保存服务地址和模型。</p>
              )}
              {!!candidates.data.excluded && (
                <p className="fy-config-pack-hint">
                  {candidates.data.excluded}{" "}
                  项连接缺少可迁移字段、使用暂不支持的接口或包含本机信息，未列入可选项。
                </p>
              )}
              <ul className="fy-config-pack-list" aria-label="可导出连接">
                {candidates.data.entries.map((entry) => (
                  <li key={entry.selectionId} className="fy-config-pack-option">
                    <Checkbox
                      label={`选择 ${entry.provider.name} · ${appNames[entry.provider.app]}`}
                      checked={selection.includes(entry.selectionId)}
                      disabled={busy}
                      onCheckedChange={(checked) => {
                        forget(exported?.exportId);
                        setExported(null);
                        setSelection((current) =>
                          checked
                            ? [...current, entry.selectionId]
                            : current.filter((id) => id !== entry.selectionId),
                        );
                      }}
                    />
                    <div>
                      <strong>{entry.provider.name}</strong>
                      <span>
                        {appNames[entry.provider.app]} · {entry.provider.model}
                      </span>
                    </div>
                    {onFillModelForm && (
                      <Button
                        className="fy-config-pack-fill"
                        disabled={busy}
                        aria-label={`将 ${entry.provider.name} 填入模型配置表单`}
                        onClick={() => void fillModelForm(entry.provider)}
                      >
                        填入模型配置表单
                      </Button>
                    )}
                  </li>
                ))}
              </ul>
              <Button
                disabled={
                  busy ||
                  !selection.length ||
                  selection.length > CONFIG_PACK_MAX_ENTRIES
                }
                onClick={() =>
                  void run(async () => {
                    forget(exported?.exportId);
                    setExported(null);
                    const next = await port.previewExport(selection);
                    if (own(next.exportId)) setExported(next);
                  })
                }
              >
                生成导出内容
              </Button>
              {selection.length > CONFIG_PACK_MAX_ENTRIES && (
                <p>每次最多选择 32 项连接。</p>
              )}
            </>
          )}
          {exported && (
            <section className="fy-config-pack-section">
              <label htmlFor={`${tabsId}-export`}>导出文本（可选择复制）</label>
              <textarea
                id={`${tabsId}-export`}
                className="fy-control-input fy-config-pack-text"
                readOnly
                value={exported.text}
              />
              <Button
                disabled={busy}
                onClick={() =>
                  void run(async () => {
                    const saved = await port.saveExport(exported);
                    if (saved && alive.current) {
                      ownedPreviews.current.delete(exported.exportId);
                      setExported(null);
                      setNotice("配置文件已保存。");
                    }
                  })
                }
              >
                另存为文件
              </Button>
            </section>
          )}
        </FeatureTabPanel>
        <FeatureTabPanel
          tabsId={tabsId}
          value="import"
          active={mode === "import"}
          layout="flow"
        >
          {!result && (
            <>
              <Button
                disabled={busy}
                onClick={() =>
                  void run(async () => {
                    const next = await port.pickFile();
                    if (next !== null && alive.current) readText(next);
                  })
                }
              >
                选择 JSON 文件
              </Button>
              <label
                className="fy-config-pack-section"
                htmlFor={`${tabsId}-import`}
              >
                或粘贴配置文本（最多 64 KB）
              </label>
              <textarea
                id={`${tabsId}-import`}
                className="fy-control-input fy-config-pack-text"
                value={text}
                disabled={busy}
                maxLength={65536}
                spellCheck={false}
                onChange={(e) => readText(e.target.value)}
              />
              {preview && (
                <section
                  className="fy-config-pack-section"
                  aria-label="导入预览"
                >
                  <h3>将保存的字段</h3>
                  {!previewCurrent && (
                    <InlineNotice tone="warning">
                      选项已修改，请更新预览后再确认。
                    </InlineNotice>
                  )}
                  <p className="fy-config-pack-hint">
                    所有保存项都需要补充 API
                    Key。当前连接保持不变，之后需在对应应用设置中单独配置和应用。
                  </p>
                  {preview.entries.map((entry, index) => (
                    <section key={index} className="fy-config-pack-entry">
                      <ProviderFields provider={entry.provider} />
                      <label>
                        处理方式
                        <select
                          className="fy-control-input"
                          aria-label={`处理 ${entry.provider.name}`}
                          disabled={busy}
                          value={choices[index]?.action ?? entry.action}
                          onChange={(e) => {
                            const action = (
                              ["add", "skip", "rename", "overwrite"] as const
                            ).find((v) => v === e.target.value);
                            if (action)
                              updateChoice(index, {
                                action,
                                name:
                                  action === "rename"
                                    ? `${entry.provider.name} 副本`
                                    : null,
                              });
                          }}
                        >
                          {!entry.conflict && (
                            <option value="add">{actionNames.add}</option>
                          )}
                          <option value="skip">{actionNames.skip}</option>
                          <option value="rename">{actionNames.rename}</option>
                          {entry.canOverwrite && (
                            <option value="overwrite">
                              {actionNames.overwrite}
                            </option>
                          )}
                        </select>
                      </label>
                      {choices[index]?.action === "rename" && (
                        <label>
                          另存名称
                          <Input
                            aria-label={`另存 ${entry.provider.name}`}
                            value={choices[index].name ?? ""}
                            maxLength={80}
                            disabled={busy}
                            onChange={(e) =>
                              updateChoice(index, {
                                action: "rename",
                                name: e.target.value,
                              })
                            }
                          />
                        </label>
                      )}
                      {entry.conflict && !entry.canOverwrite && (
                        <p className="fy-config-pack-hint">
                          同名配置已存在且不能覆盖，请跳过或另存为。
                        </p>
                      )}
                      {entry.action === "overwrite" && entry.existing && (
                        <details>
                          <summary>将覆盖的草稿字段</summary>
                          <ProviderFields provider={entry.existing} />
                        </details>
                      )}
                      <p>{actionNames[entry.action]} · 待补 API Key</p>
                    </section>
                  ))}
                </section>
              )}
            </>
          )}
          {result && (
            <section aria-label="导入结果">
              <InlineNotice>
                已保存 {result.providers.length} 项未启用连接，跳过{" "}
                {result.skipped} 项。以下是保存后读取的字段。
              </InlineNotice>
              {result.providers.map((provider) => (
                <section
                  className="fy-config-pack-entry"
                  key={`${provider.app}:${provider.name}`}
                >
                  <ProviderFields provider={provider} />
                  <p>待补 API Key · 未启用</p>
                  {onFillModelForm && (
                    <Button
                      disabled={busy}
                      aria-label={`将 ${provider.name} 填入模型配置表单`}
                      onClick={() => void fillModelForm(provider)}
                    >
                      填入模型配置表单
                    </Button>
                  )}
                </section>
              ))}
              {onFillModelForm && (
                <p className="fy-config-pack-hint">
                  填入后需补充 API Key，并在模型配置中预览、确认应用。
                </p>
              )}
              <Button
                onClick={() => {
                  setResult(null);
                  setPreview(null);
                  setText("");
                  setChoices([]);
                }}
              >
                继续导入
              </Button>
            </section>
          )}
        </FeatureTabPanel>
      </div>
    </Dialog>
  );
}

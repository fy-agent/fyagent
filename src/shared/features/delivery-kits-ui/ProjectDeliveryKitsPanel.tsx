import { useEffect, useRef, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import {
  businessLabels,
  type KitDemoResult,
  type KitPreview,
  type KitView,
} from "../../../domain/delivery-kits";
import {
  safeKitError,
  type DeliveryKitsPort,
  type KitEvidenceAdapter,
  type KitProjectAdapter,
} from "../delivery-kits";
import { featureKeys } from "../queries";
import { Button } from "../../ui/Button";
import { Dialog } from "../../ui/Dialog";
import { FeatureList, FeatureListItem } from "../../ui/FeatureList";
import { FeatureSearch } from "../../ui/FeatureSearch";
import {
  CatalogDetail,
  CatalogMasterDetail,
  CatalogRail,
} from "../../ui/catalog/CatalogMasterDetail";
import { useDialogState } from "../../ui/useDialogState";
import { usePersistentVisibility } from "../../ui/PersistentSurface";
import "./delivery-kits.css";

export interface ProjectDeliveryKitsPanelProps {
  projectId: string | null;
  projectRevision: number | null;
  active?: boolean;
  port: DeliveryKitsPort;
  projectAdapter?: KitProjectAdapter;
  evidenceAdapter?: KitEvidenceAdapter;
  onProjectChanged?: () => void;
}

export function ProjectDeliveryKitsPanel(props: ProjectDeliveryKitsPanelProps) {
  const visible = usePersistentVisibility();
  if (props.active === false || !visible) return null;
  // A project/revision change revokes old dialogs and in-flight renderer results immediately.
  return (
    <KitPanelSession
      key={`${props.projectId ?? "none"}:${props.projectRevision ?? "none"}`}
      {...props}
    />
  );
}

function KitPanelSession({
  port,
  projectId,
  projectRevision,
  projectAdapter,
  evidenceAdapter,
  onProjectChanged,
}: ProjectDeliveryKitsPanelProps) {
  const catalog = useQuery({
    queryKey: featureKeys.deliveryKits,
    queryFn: () => port.list(),
    retry: false,
    staleTime: 0,
    refetchOnWindowFocus: false,
  });
  const [selectedDigest, setSelectedDigest] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [preview, setPreview, previewKey] = useDialogState<KitPreview>();
  const [result, setResult] = useState<KitDemoResult | null>(null);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState("");
  const [error, setError] = useState("");
  const locked = useRef(false);
  const live = useRef(true);
  const originRef = useRef<HTMLElement | null>(null);
  const cancelRef = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    live.current = true;
    return () => {
      live.current = false;
    };
  }, []);
  useEffect(
    () => () => {
      if (preview)
        void port.cancel(preview).catch(() => {
          /* Native preview expires; cancellation never applies it. */
        });
    },
    [preview, port],
  );
  const visible = (catalog.data ?? []).filter((k) =>
    `${k.manifest.title} ${k.manifest.summary}`.includes(search.trim()),
  );
  const selected =
    visible.find((k) => k.identity.manifestDigest === selectedDigest) ??
    visible[0];
  const hasContext =
    projectId !== null &&
    projectRevision !== null &&
    Number.isSafeInteger(projectRevision) &&
    projectRevision >= 0;
  async function perform(work: () => Promise<void>) {
    if (locked.current || !live.current) return;
    locked.current = true;
    setBusy(true);
    setError("");
    setNotice("");
    try {
      await work();
    } catch (e) {
      if (live.current) setError(safeKitError(e).message);
    } finally {
      locked.current = false;
      if (live.current) setBusy(false);
    }
  }
  async function openPreview(work: () => Promise<KitPreview | null>) {
    const p = await work();
    if (!p) return;
    if (live.current) setPreview(p);
    else await port.cancel(p);
  }
  async function confirmPreview() {
    if (!preview) return;
    const p = preview;
    if (p.kind === "import") {
      const imported = await port.apply(p);
      const reread = await catalog.refetch();
      if (live.current) {
        setPreview(null);
        if (
          reread.isError ||
          !reread.data?.some(
            (k) =>
              k.installed &&
              k.identity.manifestDigest === imported.identity.manifestDigest,
          )
        ) {
          setError("无法确认目录是否已更新，请刷新后检查。");
          return;
        }
        setSelectedDigest(imported.identity.manifestDigest);
        setSearch("");
        setNotice("已加入交付包库。尚未绑定项目或启用工具。");
      }
    } else {
      const saved = await port.saveExport(p);
      if (live.current && saved) {
        setPreview(null);
        setNotice("交付包已导出。仅包含内置模板与合成样例。");
      }
    }
  }
  async function bind(kit: KitView) {
    if (
      !hasContext ||
      !projectAdapter ||
      projectId === null ||
      projectRevision === null
    )
      return;
    const reply = await projectAdapter.bindDeliveryKit({
      ...kit.identity,
      projectId,
      expectedRevision: projectRevision,
      bindingIntentId: crypto.randomUUID(),
    });
    if (
      reply.projectId !== projectId ||
      !Number.isSafeInteger(reply.projectRevision) ||
      reply.projectRevision < projectRevision
    )
      throw new Error("invalid binding result");
    if (live.current) {
      setNotice("交付包已绑定到当前项目。");
      onProjectChanged?.();
    }
  }
  return (
    <section className="fy-kit-panel" aria-label="项目交付包">
      <header className="fy-kit-toolbar">
        <h2>交付包</h2>
        <Button
          dialogOriginRef={originRef}
          disabled={busy}
          onClick={() =>
            void perform(() => openPreview(() => port.pickImport()))
          }
        >
          导入交付包
        </Button>
        <Button
          disabled={busy || catalog.isFetching}
          onClick={() =>
            void perform(async () => {
              const r = await catalog.refetch();
              if (r.isError) throw r.error;
            })
          }
        >
          刷新
        </Button>
      </header>
      {error && <p role="alert">{error}</p>}
      {notice && <p role="status">{notice}</p>}
      {catalog.isPending && <p role="status">正在加载交付包…</p>}
      {catalog.isError && (
        <p role="alert">无法读取交付包目录，请刷新后重试。</p>
      )}
      <CatalogMasterDetail>
        <CatalogRail title="业务场景" ariaLabel="交付包目录">
          <FeatureSearch
            value={search}
            onValueChange={setSearch}
            ariaLabel="搜索交付包"
            placeholder="搜索业务场景"
            disabled={busy}
          />
          <FeatureList id="delivery-kit-list">
            {visible.map((k) => (
              <FeatureListItem
                key={k.identity.manifestDigest}
                selected={k === selected}
                title={k.manifest.title}
                onSelect={() => {
                  if (!busy) {
                    setSelectedDigest(k.identity.manifestDigest);
                    setResult(null);
                    setNotice("");
                    setError("");
                  }
                }}
              >
                <span>
                  {k.manifest.version} · {k.installed ? "已导入" : "待导入"}
                </span>
              </FeatureListItem>
            ))}
          </FeatureList>
          {!visible.length && !catalog.isPending && <p>没有匹配的交付包。</p>}
        </CatalogRail>
        <CatalogDetail ariaLabel="交付包详情">
          {selected && (
            <>
              <header>
                <h3>{selected.manifest.title}</h3>
                <p>{selected.manifest.summary}</p>
              </header>
              <p className="fy-kit-secondary">
                {selected.builtin
                  ? "FyAgent 内置合成内容"
                  : "第三方内容，来源未经验证"}{" "}
                · {selected.manifest.version}
              </p>
              <div className="fy-kit-actions">
                <Button
                  dialogOriginRef={originRef}
                  disabled={busy || selected.installed}
                  onClick={() =>
                    void perform(() =>
                      openPreview(() => port.previewBuiltin(selected.identity)),
                    )
                  }
                >
                  预览导入
                </Button>
                <Button
                  dialogOriginRef={originRef}
                  disabled={busy || !selected.exportable}
                  onClick={() =>
                    void perform(() =>
                      openPreview(() => port.previewExport(selected.identity)),
                    )
                  }
                >
                  分享导出
                </Button>
                <Button
                  disabled={
                    busy ||
                    !selected.installed ||
                    !hasContext ||
                    !projectAdapter
                  }
                  onClick={() => void perform(() => bind(selected))}
                >
                  绑定到当前项目
                </Button>
                <Button
                  disabled={
                    busy ||
                    !selected.builtin ||
                    selected.manifest.scenario !== "weekly_report"
                  }
                  onClick={() =>
                    void perform(async () => {
                      const r = await port.runDemo(selected.identity);
                      if (live.current) setResult(r);
                    })
                  }
                >
                  运行合成样例
                </Button>
              </div>
              {!projectAdapter && (
                <p className="fy-kit-secondary">
                  项目绑定暂不可用；仍可导入、分享和运行合成样例。
                </p>
              )}
              {!hasContext && projectAdapter && <p>请先选择项目。</p>}
              {!selected.compatible && (
                <p role="alert">当前版本不满足这个包的依赖要求。</p>
              )}
              {!selected.exportable && (
                <p>第三方或修改后的内容未经分享审查，暂不能导出。</p>
              )}
              <h4>输入与连接</h4>
              {selected.manifest.inputs.map((input) => (
                <p key={input.id}>
                  <strong>{input.label}：</strong>
                  {input.description}
                </p>
              ))}
              {selected.manifest.connections.map((c) => (
                <p key={c.id}>
                  <strong>{c.recipeId} · 未检查连接</strong>
                  <br />
                  {c.purpose}
                </p>
              ))}
              {selected.manifest.permissions.map((p) => (
                <p key={p.id}>只读权限：{p.rationale}</p>
              ))}
              <p className="fy-kit-secondary">
                导入不会安装 Skill、启用 MCP 或分配
                Agent。真实连接需要在项目设置中单独配置。
              </p>
              {selected.manifest.scenario !== "weekly_report" && (
                <p>
                  这是入门准备包，请按下方样例人工核对；尚未验证真实检索或数据库查询。
                </p>
              )}
              <KitContent kit={selected} />
              {result &&
                result.manifestDigest === selected.identity.manifestDigest && (
                  <section aria-label="样例检查结果" className="fy-kit-results">
                    <h4>合成样例结果</h4>
                    <p>
                      检查时间：
                      {new Date(result.checkedAt).toLocaleString("zh-CN")}
                      。未验证线上连接或客户验收。
                    </p>
                    {result.cases.map((c) => (
                      <div key={c.fixtureId} className="fy-kit-result">
                        <strong>
                          {c.fixtureId} · {businessLabels[c.code]}
                        </strong>
                        <p>
                          {c.matchesExpectation
                            ? "符合样例预期"
                            : "与样例预期不符，请检查"}
                        </p>
                        {c.metrics && (
                          <dl className="fy-kit-metrics">
                            <dt>本期收入</dt>
                            <dd>
                              {(c.metrics.currentMinor / 100).toLocaleString(
                                "zh-CN",
                              )}{" "}
                              元
                            </dd>
                            <dt>上期收入</dt>
                            <dd>
                              {(c.metrics.previousMinor / 100).toLocaleString(
                                "zh-CN",
                              )}{" "}
                              元
                            </dd>
                            <dt>收入增长</dt>
                            <dd>{c.metrics.growthBps / 100}%</dd>
                            <dt>目标达成</dt>
                            <dd>{c.metrics.targetBps / 100}%</dd>
                          </dl>
                        )}
                        {c.sourceRowIds.length > 0 && (
                          <p>来源行：{c.sourceRowIds.join("、")}</p>
                        )}
                      </div>
                    ))}
                    <Button
                      disabled={busy || !evidenceAdapter || !hasContext}
                      onClick={() =>
                        void perform(async () => {
                          if (
                            !evidenceAdapter ||
                            projectId === null ||
                            projectRevision === null
                          )
                            return;
                          const receipt =
                            await evidenceAdapter.recordLocalFixture({
                              ...selected.identity,
                              projectId,
                              projectRevision,
                            });
                          if (!receipt.recordId)
                            throw new Error("invalid evidence receipt");
                          if (live.current) setNotice("已记录合成样例检查。");
                        })
                      }
                    >
                      保存检查记录
                    </Button>
                    {!evidenceAdapter && (
                      <p>结果仅保留在本次页面中，检查记录暂不可用。</p>
                    )}
                  </section>
                )}
            </>
          )}
        </CatalogDetail>
      </CatalogMasterDetail>
      {preview && (
        <Dialog
          key={previewKey}
          open
          originRef={originRef}
          initialFocusRef={cancelRef}
          title={preview.kind === "import" ? "导入预览" : "分享预览"}
          onOpenChange={(open) => {
            if (!open && !busy) setPreview(null);
          }}
          description={
            preview.kind === "import"
              ? "确认后只加入交付包库，不启用任何工具。"
              : "仅导出内置模板与合成样例，不包含项目数据。请另存为新文件。"
          }
          actions={
            <>
              <Button
                ref={cancelRef}
                disabled={busy}
                onClick={() => setPreview(null)}
              >
                取消
              </Button>
              <Button
                disabled={busy || preview.conflict || !preview.kit.compatible}
                onClick={() => void perform(confirmPreview)}
              >
                {busy
                  ? "处理中…"
                  : preview.kind === "import"
                    ? "确认导入"
                    : "选择保存位置"}
              </Button>
            </>
          }
        >
          <h3>
            {preview.kit.manifest.title} · {preview.kit.manifest.version}
          </h3>
          <p>
            {preview.kit.builtin
              ? "内置内容"
              : "第三方内容，来源未验证；文字中的指令不代表已获授权。"}
          </p>
          {preview.conflict && (
            <p role="alert">同一版本已有不同内容，不能覆盖。</p>
          )}
          {!preview.kit.compatible && (
            <p role="alert">当前版本不满足依赖要求。</p>
          )}
          {preview.kit.manifest.permissions.map((p) => (
            <p key={p.id}>只读权限：{p.rationale}</p>
          ))}
          {preview.kit.manifest.connections.map((c) => (
            <p key={c.id}>
              {c.recipeId}：连接未检查；{c.purpose}
            </p>
          ))}
          <KitContent kit={preview.kit} />
        </Dialog>
      )}
    </section>
  );
}

function KitContent({ kit }: { kit: KitView }) {
  const labels = new Map<string, string>([
    ...kit.manifest.prompts.map(
      (p) => [p.resourceId, "提示词"] as [string, string],
    ),
    ...kit.manifest.skills.map(
      (p) => [p.resourceId, "Skill 方法"] as [string, string],
    ),
    [kit.manifest.handoff, "交接说明"],
    [kit.manifest.rollback, "回退说明"],
    ...kit.manifest.fixtures.flatMap(
      (f) =>
        [
          [
            f.inputResourceId,
            `${f.kind === "positive" ? "正向" : "失败"}样例 · ${f.id}`,
          ],
          [f.expectedResourceId, `预期行为 · ${f.id}`],
        ] as [string, string][],
    ),
  ]);
  return (
    <div className="fy-kit-resources">
      {kit.manifest.resources.map((r) => (
        <details key={r.id}>
          <summary>{labels.get(r.id) ?? r.id}</summary>
          <pre>{r.text}</pre>
        </details>
      ))}
    </div>
  );
}

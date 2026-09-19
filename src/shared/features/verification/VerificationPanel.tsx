import { useEffect, useRef, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  VERIFICATION_STAGES,
  STAGE_LABELS,
  SOURCE_LABELS,
  REASON_LABELS,
  CHECKER_LABELS,
  SAMPLE_CODE_LABELS,
  ROLLBACKS,
  evidenceLabel,
  manualSchema,
  handoffSchema,
  type Stage,
  type ManualRequest,
  type VerificationSnapshot,
  type HandoffNotes,
  type HandoffPreview,
} from "@/domain/verification";
import { useFeatures } from "../provider";
import { featureKeys } from "../queries";
import { Button } from "../../ui/Button";
import { usePersistentVisibility } from "../../ui/PersistentSurface";
import "./verification.css";

const rollbackLabels = {
  review_configuration: "检查配置后手工回退",
  guarded_file_recovery: "检查文件恢复条件",
  manual_only: "由接手人手工处理",
  not_available: "尚无回退办法",
};

/** Root composes this panel in the project's tabs; it owns no global route. */
export function VerificationPanel({
  projectId,
  active = true,
  mutationBlockedReason,
}: {
  projectId: string;
  active?: boolean;
  mutationBlockedReason?: string;
}) {
  return (
    <ProjectVerification
      key={projectId}
      projectId={projectId}
      active={active}
      mutationBlockedReason={mutationBlockedReason}
    />
  );
}

function ProjectVerification({
  projectId,
  active,
  mutationBlockedReason,
}: {
  projectId: string;
  active: boolean;
  mutationBlockedReason?: string;
}) {
  const { ports } = useFeatures();
  const visible = usePersistentVisibility() && active;
  const client = useQueryClient();
  const query = useQuery({
    queryKey: featureKeys.verification(projectId),
    queryFn: () => ports.verification.get(projectId),
    enabled: visible,
    retry: false,
    staleTime: 0,
    gcTime: 0,
    refetchOnMount: "always",
  });
  const [busy, setBusy] = useState(false);
  const lock = useRef(false);
  const live = useRef(true);
  useEffect(() => {
    live.current = true;
    return () => {
      live.current = false;
    };
  }, []);
  const [error, setError] = useState("");
  const [needsReview, setNeedsReview] = useState(false);
  const [message, setMessage] = useState("");
  const [preview, setPreview] = useState<HandoffPreview | null>(null);
  const [manualOpen, setManualOpen] = useState(false);
  const [sampleCase, setSampleCase] = useState<"baseline" | "missing_field">(
    "baseline",
  );
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (!visible) return;
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, [visible]);
  async function act(
    action: () => Promise<VerificationSnapshot | void>,
    readOnly = false,
  ) {
    if (
      !visible ||
      !live.current ||
      lock.current ||
      (mutationBlockedReason && !readOnly)
    )
      return;
    lock.current = true;
    setBusy(true);
    setError("");
    setMessage("");
    setPreview(null);
    try {
      const result = await action();
      if (!live.current) return;
      if (result) {
        client.setQueryData(featureKeys.verification(projectId), result);
        setNeedsReview(false);
      }
    } catch {
      if (!live.current) return;
      setNeedsReview(true);
      setError("操作未完成，请刷新并检查配置、范围与依据后重试。");
      await client.invalidateQueries({
        queryKey: featureKeys.verification(projectId),
        refetchType: "none",
      });
    } finally {
      lock.current = false;
      if (live.current) setBusy(false);
    }
  }
  const data = query.data;
  const ready =
    visible &&
    !busy &&
    !query.isFetching &&
    !query.isError &&
    !needsReview &&
    !mutationBlockedReason &&
    data?.available === true &&
    data.projectRevision !== null;
  const expectedRevision = data?.projectRevision ?? 0;
  return (
    <section
      className="fy-verification"
      aria-label="验证与交接"
      aria-busy={busy || query.isFetching}
    >
      <header className="fy-verification-actions">
        <h2>验证与交接</h2>
        <Button
          disabled={!visible || busy}
          onClick={() =>
            void act(async () => {
              const r = await query.refetch();
              if (r.error) throw r.error;
              return r.data;
            }, true)
          }
        >
          刷新
        </Button>
      </header>
      {query.isPending && <p role="status">正在读取验证记录</p>}
      {(error || query.isError) && (
        <p role="alert">{error || "读取失败，已有结果需要重新确认。"}</p>
      )}
      {message && <p role="status">{message}</p>}
      {mutationBlockedReason && <p role="status">{mutationBlockedReason}</p>}
      {data && !data.available && (
        <p role="status">
          当前项目暂不能检查。已有记录仅供查阅，请在项目配置完成后刷新。
        </p>
      )}
      <div className="fy-verification-actions">
        <Button
          disabled={!ready}
          onClick={() =>
            void act(() =>
              ports.verification.run({
                projectId,
                expectedRevision,
                checker: "saved_configuration_readback",
                runId: crypto.randomUUID(),
                fixture: null,
              }),
            )
          }
        >
          检查保存的配置
        </Button>
        <Button
          disabled={!ready}
          onClick={() =>
            void act(() =>
              ports.verification.run({
                projectId,
                expectedRevision,
                checker: "saved_model_probe",
                runId: crypto.randomUUID(),
                fixture: null,
              }),
            )
          }
        >
          检查模型（可能产生用量）
        </Button>
        <label>
          本机样例
          <select
            aria-label="本机样例"
            disabled={!ready}
            value={sampleCase}
            onChange={(event) =>
              setSampleCase(event.target.value as "baseline" | "missing_field")
            }
          >
            <option value="baseline">正常周报</option>
            <option value="missing_field">缺少必填字段</option>
          </select>
        </label>
        <Button
          disabled={!ready}
          onClick={() =>
            void act(() =>
              ports.verification.run({
                projectId,
                expectedRevision,
                checker: "kit_validator",
                runId: crypto.randomUUID(),
                fixture: sampleCase,
              }),
            )
          }
        >
          检查本机样本
        </Button>
        {busy && (
          <Button
            disabled={!visible}
            onClick={() =>
              void ports.verification
                .cancel(projectId)
                .then(
                  (accepted) =>
                    live.current &&
                    setMessage(
                      accepted
                        ? "已请求取消，正在保存检查结果。"
                        : "本次操作已结束或不支持取消。",
                    ),
                )
                .catch(
                  () =>
                    live.current && setError("取消未完成，请等待检查结束。"),
                )
            }
          >
            取消检查
          </Button>
        )}
        <Button disabled={!ready} onClick={() => setManualOpen((v) => !v)}>
          登记检查或客户验收
        </Button>
      </div>
      <p className="fy-verification-note">
        各项结果分别记录。本机样本不证明外部工具已连接；客户验收由负责人登记。
      </p>
      <div className="fy-verification-stages">
        {VERIFICATION_STAGES.map((stage) => {
          const records =
            data?.evidence.filter((e) => e.stage === stage).reverse() ?? [];
          return (
            <section key={stage} aria-label={STAGE_LABELS[stage]}>
              <h3>{STAGE_LABELS[stage]}</h3>
              {records.length === 0 ? (
                <p>未检查</p>
              ) : (
                records.map((e) => (
                  <article key={e.id} className="fy-verification-record">
                    <strong>
                      {query.isError || needsReview
                        ? "待复核"
                        : evidenceLabel(e, now)}
                    </strong>{" "}
                    · {SOURCE_LABELS[e.sourceClass]}
                    <p>{REASON_LABELS[e.reasonCode]}</p>
                    {e.fixture && (
                      <p>
                        样例：
                        {e.fixture === "baseline" ? "正常周报" : "缺少必填字段"}
                      </p>
                    )}
                    {e.sample && (
                      <p>
                        {SAMPLE_CODE_LABELS[e.sample.code]}；
                        {e.sample.matchesExpectation
                          ? "符合样例预期"
                          : "与样例预期不符"}
                      </p>
                    )}
                    {e.sample?.metrics && (
                      <p>
                        本期金额{" "}
                        {(e.sample.metrics.currentMinor / 100).toLocaleString(
                          "zh-CN",
                          {
                            minimumFractionDigits: 2,
                            maximumFractionDigits: 2,
                          },
                        )}{" "}
                        元； 上期金额{" "}
                        {(e.sample.metrics.previousMinor / 100).toLocaleString(
                          "zh-CN",
                          {
                            minimumFractionDigits: 2,
                            maximumFractionDigits: 2,
                          },
                        )}{" "}
                        元； 增长{" "}
                        {(e.sample.metrics.growthBps / 100).toFixed(2)}%；
                        目标完成 {(e.sample.metrics.targetBps / 100).toFixed(2)}
                        %
                      </p>
                    )}
                    {!!e.sample?.sourceRowIds.length && (
                      <p>来源行：{e.sample.sourceRowIds.join("、")}</p>
                    )}
                    <p>
                      检查时间：
                      <time dateTime={e.observedAt}>
                        {new Date(e.observedAt).toLocaleString()}
                      </time>
                    </p>
                    {e.expiresAt && (
                      <p>有效至：{new Date(e.expiresAt).toLocaleString()}</p>
                    )}
                    <details>
                      <summary>来源与版本</summary>
                      <p>
                        应用 {e.appVersion} · {CHECKER_LABELS[e.checkerId]} v
                        {e.checkerVersion}
                      </p>
                      <p>
                        项目版本 {e.projectRevision}
                        {e.kit ? ` · 交付包版本 ${e.kit.kitVersion}` : ""}
                      </p>
                      {e.manual && (
                        <>
                          <p>
                            {e.manual.person}（{e.manual.role}） ·{" "}
                            {e.manual.scope}
                          </p>
                          {e.manual.externalBasis && (
                            <p>
                              依据：{e.manual.externalBasis.reference} ·{" "}
                              {e.manual.externalBasis.issuer}
                            </p>
                          )}
                        </>
                      )}
                    </details>
                    {e.validity !== "revoked" && (
                      <Button
                        disabled={!visible || busy || !!mutationBlockedReason}
                        onClick={() =>
                          void act(() =>
                            ports.verification.revoke({
                              projectId,
                              evidenceId: e.id,
                            }),
                          )
                        }
                      >
                        撤销这条记录
                      </Button>
                    )}
                  </article>
                ))
              )}
            </section>
          );
        })}
      </div>
      {manualOpen && data && (
        <ManualForm
          snapshot={data}
          disabled={!ready}
          onSubmit={(request) =>
            act(async () => {
              const result = await ports.verification.record(request);
              if (live.current) setManualOpen(false);
              return result;
            })
          }
        />
      )}
      {data && (
        <HandoffEditor
          key={data.handoffRevision}
          initial={data.handoff}
          disabled={!ready}
          onSave={(handoff) =>
            act(() =>
              ports.verification.saveHandoff({
                projectId,
                expectedRevision,
                handoffRevision: data.handoffRevision,
                handoff,
              }),
            )
          }
        />
      )}
      <div className="fy-verification-actions">
        <Button
          disabled={!visible || busy || !data}
          onClick={() =>
            void act(async () => {
              const result = await ports.verification.preview(projectId);
              if (live.current) setPreview(result);
            }, true)
          }
        >
          预览脱敏交接
        </Button>
      </div>
      {preview && visible && (
        <section aria-label="交接预览">
          <h3>交接预览</h3>
          <p>
            包含登记人、接手人和结构化事项；不包含原始配置、响应内容或凭据。导出前将再次检查当前状态。
          </p>
          <pre>{preview.markdown}</pre>
          <details>
            <summary>JSON</summary>
            <pre>{preview.json}</pre>
          </details>
          <div className="fy-verification-actions">
            {(["json", "markdown"] as const).map((format) => (
              <Button
                disabled={busy}
                key={format}
                onClick={() =>
                  void act(async () => {
                    const saved = await ports.verification.export(
                      projectId,
                      format,
                    );
                    if (live.current)
                      setMessage(saved ? "交接文件已导出。" : "已取消导出。");
                  }, true)
                }
              >
                导出 {format === "json" ? "JSON" : "Markdown"}
              </Button>
            ))}
          </div>
        </section>
      )}
    </section>
  );
}

function ManualForm({
  snapshot,
  disabled,
  onSubmit,
}: {
  snapshot: VerificationSnapshot;
  disabled: boolean;
  onSubmit: (request: ManualRequest) => Promise<void>;
}) {
  const [stage, setStage] = useState<Stage>("customer_accepted");
  const [outcome, setOutcome] = useState<ManualRequest["outcome"]>("passed");
  const [person, setPerson] = useState("");
  const [role, setRole] = useState("");
  const [scope, setScope] = useState("");
  const [reference, setReference] = useState("");
  const [issuer, setIssuer] = useState("");
  const [basis, setBasis] = useState<string[]>([]);
  const [formError, setFormError] = useState("");
  const [at, setAt] = useState(() =>
    new Date(Date.now() - new Date().getTimezoneOffset() * 60000)
      .toISOString()
      .slice(0, 23),
  );
  const eligible = snapshot.evidence.filter(
    (e) =>
      e.outcome === "passed" &&
      e.validity === "current" &&
      evidenceLabel(e) === "通过",
  );
  return (
    <form
      className="fy-verification-form"
      aria-label="人工登记"
      onSubmit={(event) => {
        event.preventDefault();
        if (disabled) return;
        const observedAt = new Date(at);
        const parsed = manualSchema.safeParse({
          projectId: snapshot.projectId,
          expectedRevision: snapshot.projectRevision ?? 0,
          stage,
          outcome,
          person,
          role,
          scope,
          observedAt: Number.isFinite(observedAt.getTime())
            ? observedAt.toISOString()
            : "",
          externalBasis: reference || issuer ? { reference, issuer } : null,
          basisEvidenceIds: basis,
        });
        if (!parsed.success) {
          const field = parsed.error.issues[0]?.path.join(".") ?? "";
          const labels: Record<string, string> = {
            person: "登记或验收人",
            role: "角色",
            scope: "作用范围",
            "externalBasis.issuer": "出具记录的组织或人员",
          };
          setFormError(
            field === "observedAt"
              ? "发生时间：请填写有效日期和时间。"
              : field === "externalBasis.reference"
                ? "记录编号或文档链接：请填写记录编号或不含登录信息、查询参数的 HTTPS 文档链接。"
                : labels[field]
                  ? `${labels[field]}：请填写文字、数字或常用中文标点，不含链接、路径或凭据，且不要留首尾空格。`
                  : "请补全登记内容，并填写外部依据或选择已有记录。",
          );
          return;
        }
        setFormError("");
        void onSubmit(parsed.data);
      }}
    >
      <h3>人工登记</h3>
      {formError && <p role="alert">{formError}</p>}
      <p>
        填写可核对的记录编号或文档链接，或选择已有依据。仅本机模拟样本不能作为客户验收。
      </p>
      <fieldset disabled={disabled}>
        <label>
          阶段
          <select
            value={stage}
            onChange={(e) => setStage(e.target.value as Stage)}
          >
            {VERIFICATION_STAGES.map((v) => (
              <option key={v} value={v}>
                {STAGE_LABELS[v]}
              </option>
            ))}
          </select>
        </label>
        <label>
          结果
          <select
            value={outcome}
            onChange={(e) =>
              setOutcome(e.target.value as ManualRequest["outcome"])
            }
          >
            <option value="passed">通过</option>
            <option value="failed">失败</option>
            <option value="unknown">未确认</option>
          </select>
        </label>
        <label>
          登记或验收人
          <input
            required
            maxLength={80}
            value={person}
            onChange={(e) => setPerson(e.target.value)}
          />
        </label>
        <label>
          角色
          <input
            required
            maxLength={80}
            value={role}
            onChange={(e) => setRole(e.target.value)}
          />
        </label>
        <label>
          作用范围
          <input
            required
            maxLength={160}
            value={scope}
            onChange={(e) => setScope(e.target.value)}
          />
        </label>
        <label>
          发生时间
          <input
            required
            type="datetime-local"
            step="0.001"
            value={at}
            onChange={(e) => setAt(e.target.value)}
          />
        </label>
        <label>
          记录编号或文档链接
          <input
            maxLength={2048}
            value={reference}
            onChange={(e) => setReference(e.target.value)}
            placeholder="UAT-2026-09 或 HTTPS 文档链接"
          />
        </label>
        <label>
          出具记录的组织或人员
          <input
            maxLength={160}
            value={issuer}
            onChange={(e) => setIssuer(e.target.value)}
          />
        </label>
        {eligible.map((e) => (
          <label key={e.id} className="fy-verification-check">
            <input
              type="checkbox"
              checked={basis.includes(e.id)}
              onChange={(event) =>
                setBasis((v) =>
                  event.target.checked
                    ? [...v, e.id]
                    : v.filter((id) => id !== e.id),
                )
              }
            />
            {STAGE_LABELS[e.stage]} · {SOURCE_LABELS[e.sourceClass]} ·{" "}
            {new Date(e.observedAt).toLocaleString()}
          </label>
        ))}
        <Button
          type="submit"
          disabled={disabled || (!basis.length && (!reference || !issuer))}
        >
          保存人工记录
        </Button>
      </fieldset>
    </form>
  );
}

function HandoffEditor({
  initial,
  disabled,
  onSave,
}: {
  initial: HandoffNotes;
  disabled: boolean;
  onSave: (notes: HandoffNotes) => Promise<void>;
}) {
  const [notes, setNotes] = useState(initial);
  const [formError, setFormError] = useState("");
  return (
    <form
      className="fy-verification-form"
      aria-label="交接事项"
      onSubmit={(e) => {
        e.preventDefault();
        if (disabled) return;
        const parsed = handoffSchema.safeParse(notes);
        if (!parsed.success) {
          const path = parsed.error.issues[0]?.path ?? [];
          const field = path[2] === "owner" ? "责任人" : "事项";
          setFormError(
            typeof path[1] === "number"
              ? `${field} ${path[1] + 1}：请填写文字、数字或常用中文标点，不含链接、路径或凭据，且不要留首尾空格。`
              : "请检查交接事项和回退办法后再保存。",
          );
          return;
        }
        setFormError("");
        void onSave(parsed.data);
      }}
    >
      <h3>交接事项</h3>
      {formError && <p role="alert">{formError}</p>}
      <fieldset disabled={disabled}>
        {notes.items.map((item, index) => (
          <div className="fy-verification-item" key={index}>
            <label>
              事项 {index + 1}
              <input
                required
                maxLength={160}
                value={item.title}
                onChange={(e) =>
                  setNotes((v) => ({
                    ...v,
                    items: v.items.map((i, n) =>
                      n === index ? { ...i, title: e.target.value } : i,
                    ),
                  }))
                }
              />
            </label>
            <label>
              责任人 {index + 1}
              <input
                maxLength={80}
                value={item.owner ?? ""}
                placeholder="未指定"
                onChange={(e) =>
                  setNotes((v) => ({
                    ...v,
                    items: v.items.map((i, n) =>
                      n === index ? { ...i, owner: e.target.value || null } : i,
                    ),
                  }))
                }
              />
            </label>
            <label className="fy-verification-check">
              <input
                type="checkbox"
                checked={item.completed}
                onChange={(e) =>
                  setNotes((v) => ({
                    ...v,
                    items: v.items.map((i, n) =>
                      n === index ? { ...i, completed: e.target.checked } : i,
                    ),
                  }))
                }
              />
              已完成
            </label>
            <Button
              onClick={() =>
                setNotes((v) => ({
                  ...v,
                  items: v.items.filter((_, n) => n !== index),
                }))
              }
            >
              移除事项 {index + 1}
            </Button>
          </div>
        ))}
        <Button
          disabled={disabled || notes.items.length >= 100}
          onClick={() =>
            setNotes((v) => ({
              ...v,
              items: [...v.items, { title: "", owner: null, completed: false }],
            }))
          }
        >
          添加待办
        </Button>
        <label>
          回退办法
          <select
            value={notes.rollback ?? "not_available"}
            onChange={(e) =>
              setNotes((v) => ({
                ...v,
                rollback: e.target.value as HandoffNotes["rollback"],
              }))
            }
          >
            {ROLLBACKS.map((v) => (
              <option value={v} key={v}>
                {rollbackLabels[v]}
              </option>
            ))}
          </select>
        </label>
        <p>文件恢复需重新检查条件，不会撤销账户授权或远端业务。</p>
        <Button type="submit" disabled={disabled}>
          保存交接事项
        </Button>
      </fieldset>
    </form>
  );
}

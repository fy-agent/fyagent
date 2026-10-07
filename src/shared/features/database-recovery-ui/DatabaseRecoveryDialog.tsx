import { useId, useRef } from "react";

import { Button } from "../../ui/Button";
import { Dialog } from "../../ui/Dialog";
import type { DialogOriginRef } from "../../ui/dialogOrigin";
import { usePersistentVisibility } from "../../ui/PersistentSurface";
import { InlineNotice, Spinner } from "../../ui/primitives";
import type {
  DatabaseRecoveryPort,
  DatabaseRestoreOutcome,
} from "../database-recovery";
import { useDatabaseRecoverySession } from "./useDatabaseRecoverySession";

import "./database-recovery.css";

const failureCopy: Record<DatabaseRestoreOutcome["resultCode"], string> = {
  restored:
    "数据库已恢复并完成读取检查。请重新打开相关页面，检查账号、模型和历史记录。",
  precheckFailed:
    "备份检查未通过，当前数据库未被替换。请确认所选备份仍可读取。",
  candidateFailed:
    "备份未能完成恢复准备，当前数据库未被替换。请保留原备份，检查版本兼容性和磁盘空间。",
  archiveFailed:
    "旧历史记录未能完成归档，当前数据库未被替换。请检查磁盘空间和写入权限。",
  safetyBackupFailed:
    "未能建立可读取的恢复前备份，当前数据库未被替换。请检查磁盘空间和备份目录权限。",
  publishFailed: "数据库替换未完成。",
  readbackFailed:
    "数据库已替换，但读取检查未完成。请保留恢复前备份，刷新检查，勿重复恢复。",
  workerLost: "恢复操作中断，未能取得完整结果。",
};

function resultCopy(outcome: DatabaseRestoreOutcome): string {
  if (outcome.publication === "unknown") {
    return "无法确认数据库是否已替换。请保留现有数据库和备份，刷新检查，勿重复恢复。";
  }
  if (outcome.resultCode === "workerLost") {
    return "数据库已替换，但恢复操作中断，读取检查未完成。请保留恢复前备份，刷新检查，勿重复恢复。";
  }
  if (outcome.resultCode === "publishFailed") {
    return "数据库替换未完成，已确认当前数据库未被替换。处理故障后可重试一次。";
  }
  return failureCopy[outcome.resultCode];
}

/** The composition owner must retain this controlled component across closes. */
export function DatabaseRecoveryDialog({
  open,
  originRef,
  onClose,
  port,
}: {
  open: boolean;
  originRef: DialogOriginRef | undefined;
  onClose: () => void;
  port: DatabaseRecoveryPort;
}) {
  const visible = usePersistentVisibility();
  const session = useDatabaseRecoverySession(port, open && visible);
  const cancelRef = useRef<HTMLButtonElement>(null);
  const selectionId = useId();
  const close = () => {
    session.revoke();
    onClose();
  };

  return (
    <Dialog
      open={open}
      originRef={originRef}
      initialFocusRef={cancelRef}
      onOpenChange={(next) => {
        if (!next) close();
      }}
      title="本机备份与恢复"
      description="恢复会用所选 .db 副本替换本机数据库。替换前会保存当前数据库副本；最新本机恢复记录会保留。"
      size="comfortable"
      presentationKey={
        session.restoring
          ? "restoring"
          : session.outcome || session.unconfirmed
            ? "result"
            : "select"
      }
      actions={
        <>
          <Button ref={cancelRef} onClick={close}>
            关闭
          </Button>
          <Button
            disabled={session.loading || session.restoring}
            onClick={() => void session.refresh()}
          >
            刷新备份
          </Button>
          {(session.outcome || session.unconfirmed) && !session.restoring && (
            <Button
              disabled={session.checking}
              onClick={() => void session.checkReadability()}
            >
              {session.checking ? "正在检查…" : "检查当前数据库"}
            </Button>
          )}
          {!session.writesBlocked && !session.retryExhausted && (
            <Button
              className="fy-control-button-primary"
              disabled={!session.canRestore}
              onClick={() => void session.restore()}
            >
              {session.restoring
                ? "正在恢复…"
                : session.outcome
                  ? "重试恢复"
                  : "确认恢复"}
            </Button>
          )}
        </>
      }
    >
      <div className="fy-database-recovery">
        {session.loading && <Spinner label="正在读取本机备份" />}
        {session.listError && (
          <InlineNotice tone="warning">
            无法读取本机备份列表，请刷新后检查。
          </InlineNotice>
        )}
        {!session.loading &&
          !session.listError &&
          session.backups.length === 0 && <p>没有可用的本机 .db 备份。</p>}
        {session.backups.length > 0 && (
          <fieldset
            className="fy-database-recovery-list"
            disabled={
              session.loading ||
              session.restoring ||
              session.writesBlocked ||
              session.retryExhausted
            }
          >
            <legend>选择本机 .db 备份</legend>
            {session.backups.map((entry) => (
              <label
                className="fy-database-recovery-option"
                key={entry.filename}
              >
                <input
                  type="radio"
                  name={selectionId}
                  checked={session.selected === entry.filename}
                  onChange={() => session.select(entry.filename)}
                />
                <span>
                  <code>{entry.filename}</code>
                  <small>
                    {entry.createdAt
                      ? new Date(entry.createdAt).toLocaleString("zh-CN")
                      : "时间未知"}{" "}
                    · {entry.sizeBytes.toLocaleString("zh-CN")} 字节
                  </small>
                </span>
              </label>
            ))}
          </fieldset>
        )}
        {session.selected && !session.writesBlocked && !session.restoring && (
          <p>
            确认后将恢复 <code>{session.selected}</code>
            。原副本会保留，本机凭据仍受当前设备绑定限制。
          </p>
        )}
        {session.restoring && (
          <InlineNotice>
            正在恢复 <code>{session.operationFilename}</code>
            。关闭对话不会取消已经开始的恢复。
          </InlineNotice>
        )}
        {session.outcome && (
          <InlineNotice
            tone={
              session.outcome.resultCode === "restored" &&
              session.outcome.warnings.length === 0
                ? "info"
                : "warning"
            }
          >
            <p>{resultCopy(session.outcome)}</p>
            <p>
              所选副本：<code>{session.operationFilename}</code>
            </p>
            {session.outcome.safetyBackupFilename && (
              <p>
                恢复前备份：<code>{session.outcome.safetyBackupFilename}</code>
              </p>
            )}
            {session.outcome.warnings.includes("retentionFailed") && (
              <p>
                备份清理未完成，恢复前备份仍已保留。请保留这些文件，检查备份目录。
              </p>
            )}
          </InlineNotice>
        )}
        {session.unconfirmed && !session.restoring && (
          <InlineNotice tone="warning">
            无法确认数据库是否已替换。请保留现有数据库和备份，刷新检查，勿重复恢复。
          </InlineNotice>
        )}
        {session.retryExhausted &&
          session.outcome?.publication === "notCommitted" && (
            <p>
              本次重试已用完。请保留现有数据库和备份，处理故障后重新启动应用再检查。
            </p>
          )}
        {session.writesBlocked && !session.restoring && (
          <p>
            刷新仅重新读取备份列表，不会再次替换数据库；列表可读不能确认数据库内容已恢复。
          </p>
        )}
        {session.readability && (
          <InlineNotice
            tone={session.readability.state === "readable" ? "info" : "warning"}
          >
            {session.readability.state === "readable"
              ? "当前数据库可读取。此检查不能确认它已恢复为所选副本，原恢复结果仍保留。"
              : "无法确认当前数据库可读取。请保留数据库和备份，检查磁盘与权限后再检查。"}
          </InlineNotice>
        )}
        <details className="fy-database-recovery-types">
          <summary>文件用途与限制</summary>
          <dl>
            <dt>配置迁移包</dt>
            <dd>
              迁移可移植的连接配置，不包含 API Key、账户绑定和本机专属设置。
            </dd>
            <dt>私有 .db 副本</dt>
            <dd>
              无损保存本机数据库，可能含登录凭据，请勿分享；恢复时会保留最新本机恢复记录。
            </dd>
            <dt>脱敏 SQL</dt>
            <dd>用于导出可迁移数据，不包含完整凭据，不能代替私有 .db 副本。</dd>
            <dt>云快照</dt>
            <dd>
              用于跨设备同步；本机绑定的凭据和恢复记录不能直接在其他设备复用。
            </dd>
          </dl>
        </details>
      </div>
    </Dialog>
  );
}

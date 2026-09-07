import type { FileWriteTarget } from "../file-writes";
import { CopyablePath } from "./CopyablePath";

import "./FileWriteDisclosure.css";

export function FileWriteDisclosure({
  targets,
  preservedPaths = [],
}: {
  targets: readonly FileWriteTarget[];
  preservedPaths?: readonly string[];
}) {
  return (
    <div className="fy-file-write-disclosure">
      {targets.map((target) => (
        <div className="fy-file-write-target" key={target.path}>
          <div className="fy-file-write-path-row">
            <span className="fy-file-write-path-label">
              {target.exists ? "将修改" : "将创建"}
            </span>
            <CopyablePath label="配置文件路径" value={target.path} />
          </div>
          <div className="fy-file-write-path-row">
            <span className="fy-file-write-path-label">备份位置</span>
            <CopyablePath label="备份文件路径" value={target.backupPath} />
          </div>
          {!target.exists ? (
            <p>文件尚不存在，不生成原文件备份；本次撤回将删除新创建的文件。</p>
          ) : null}
        </div>
      ))}
      {preservedPaths.map((path) => (
        <div className="fy-file-write-path-row" key={path}>
          <span className="fy-file-write-path-label">保持不变</span>
          <CopyablePath label="不修改的文件路径" value={path} />
        </div>
      ))}
      {targets.length > 0 ? (
        <p>
          写入前会备份原文件，每个文件保留最近一次修改前的内容，需要时可用备份恢复原文件。
          使用“撤回文件修改”时，文件被其他程序修改后会停止恢复；手动恢复前请先退出相关软件并检查后续改动。
          认证备份可能包含登录凭证，请勿分享。
        </p>
      ) : (
        <p>本次不修改软件的认证或配置文件。</p>
      )}
    </div>
  );
}

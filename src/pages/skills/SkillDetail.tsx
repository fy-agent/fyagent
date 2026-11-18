import { CaretDownIcon } from "@phosphor-icons/react/dist/csr/CaretDown";
import { useState } from "react";

import { skillInstallPath } from "../../shared/features/helpers";
import {
  SKILL_TARGETS,
  type InstalledSkill,
  type SkillTargetId,
} from "../../shared/features/types";
import { Button } from "../../shared/ui/Button";
import {
  Collapsible,
  CollapsibleCaret,
  CollapsibleContent,
  CollapsibleTrigger,
} from "../../shared/ui/Collapsible";
import type { DialogOriginRef } from "../../shared/ui/dialogOrigin";
import { Badge, InlineNotice } from "../../shared/ui/primitives";
import { AssignmentPanel } from "../../shared/ui/AssignmentPanel";
import { CopyablePath } from "../../shared/features/controls/CopyablePath";
import { ExternalLinkButton } from "../../shared/features/controls/ExternalLinkButton";
import {
  formatSkillTimestamp,
  githubRepoUrl,
  isMarketSkill,
} from "./skillsShared";

export function SkillDetail({
  originRef,
  skill,
  update,
  busy,
  onToggle,
  onUpdate,
  onUninstall,
  showAssignment,
}: {
  originRef?: DialogOriginRef;
  skill: InstalledSkill;
  update?: { remoteHash: string };
  busy: boolean;
  onToggle: (app: SkillTargetId, enabled: boolean) => void;
  onUpdate: () => void;
  onUninstall: () => void;
  showAssignment: boolean;
}) {
  const [installationOpen, setInstallationOpen] = useState(false);
  const repo =
    skill.repoOwner && skill.repoName
      ? `${skill.repoOwner}/${skill.repoName}`
      : null;
  const market = isMarketSkill(skill);
  const repoUrl =
    skill.repoOwner && skill.repoName
      ? githubRepoUrl(skill.repoOwner, skill.repoName)
      : null;
  const sourceLabel = market ? "Skill 市场" : repo ? "GitHub 仓库" : "本地导入";
  const description = skill.description?.trim();

  return (
    <section
      className="fy-feature-panel fy-feature-detail fy-feature-detail-scroll"
      aria-label="Skill 详情"
    >
      <div className="fy-feature-detail-header">
        <div className="fy-feature-detail-title">
          <h2>{skill.name}</h2>
          {update && <Badge tone="warning">有更新</Badge>}
          <Badge tone={repo ? "accent" : "neutral"}>{sourceLabel}</Badge>
        </div>
        {description && <p className="fy-feature-intro">{description}</p>}
        {skill.readOnly || skill.readOnlyTargets?.length ? (
          <InlineNotice tone="warning">
            检测到链接目录：{skill.readOnly ? "此 Skill 来源只读。" : ""}
            {skill.readOnlyTargets?.length
              ? `只读目标：${skill.readOnlyTargets.map((id) => SKILL_TARGETS.find((target) => target.id === id)?.label ?? id).join("、")}。`
              : ""}
            链接及其目标不会写入或删除；请在外部调整链接后刷新。
          </InlineNotice>
        ) : null}
        <div className="fy-feature-actions">
          {update && (
            <Button
              className="fy-control-button-primary"
              disabled={
                busy || skill.readOnly || Boolean(skill.readOnlyTargets?.length)
              }
              onClick={onUpdate}
            >
              更新
            </Button>
          )}
          <Button
            className="fy-control-button-danger"
            disabled={
              busy || skill.readOnly || Boolean(skill.readOnlyTargets?.length)
            }
            onClick={onUninstall}
            dialogOriginRef={originRef}
          >
            卸载
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
                {repo && !market && (
                  <>
                    <dt>仓库</dt>
                    <dd>{repo}</dd>
                  </>
                )}
                {skill.repoBranch && !market && (
                  <>
                    <dt>分支</dt>
                    <dd>{skill.repoBranch}</dd>
                  </>
                )}
                <dt>安装目录</dt>
                <dd>
                  <CopyablePath
                    revealValue={false}
                    value={skillInstallPath(skill)}
                  />
                </dd>
                {skill.installedAt > 0 && (
                  <>
                    <dt>安装时间</dt>
                    <dd>{formatSkillTimestamp(skill.installedAt)}</dd>
                  </>
                )}
                <dt>最近更新</dt>
                <dd>{formatSkillTimestamp(skill.updatedAt)}</dd>
              </dl>
              {(repoUrl || skill.readmeUrl) && (
                <div className="fy-feature-actions">
                  {repoUrl && (
                    <ExternalLinkButton url={repoUrl}>
                      打开仓库
                    </ExternalLinkButton>
                  )}
                  {skill.readmeUrl && (
                    <ExternalLinkButton url={skill.readmeUrl}>
                      查看说明
                    </ExternalLinkButton>
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
            apps={skill.apps}
            disabled={busy}
            disabledTargets={SKILL_TARGETS.filter((target) =>
              skill.readOnlyTargets?.includes(target.id),
            ).map((target) => target.id)}
            labelSuffix="Skill 分配"
            onToggle={onToggle}
            targets={SKILL_TARGETS}
          />
        </div>
      )}
    </section>
  );
}

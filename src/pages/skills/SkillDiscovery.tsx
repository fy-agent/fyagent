import { useEffect, useMemo, useRef, useState } from "react";

import {
  errorMessage,
  isDiscoverableInstalled,
  skillInstallDestination,
} from "../../shared/features/helpers";
import {
  useInstalledSkills,
  useSkillHubSearch,
} from "../../shared/features/queries";
import {
  SKILL_DISCOVERY_PAGE_SIZE,
  SKILLHUB_CATEGORY_ALL,
  SKILLHUB_CATEGORY_TABS,
  type SkillHubCategoryFilter,
  type SkillTargetId,
} from "../../shared/features/types";
import { Button } from "../../shared/ui/Button";
import { AnimatePresence } from "../../shared/ui/motion";
import type { DialogOriginRef } from "../../shared/ui/dialogOrigin";
import { useDialogState } from "../../shared/ui/useDialogState";
import { Dialog } from "../../shared/ui/Dialog";
import {
  Badge,
  EmptyState,
  InlineNotice,
  Spinner,
} from "../../shared/ui/primitives";
import { InstallTargetDialog } from "../../shared/features/controls/InstallTargetDialog";
import { ExternalLinkButton } from "../../shared/features/controls/ExternalLinkButton";
import { FeaturePagination } from "../../shared/ui/FeaturePagination";
import { FeatureSearch } from "../../shared/ui/FeatureSearch";
import { FeatureTabs } from "../../shared/ui/FeatureTabs";
import {
  isMarketSkill,
  skillCardBody,
  skillDetailMeta,
  skillDocsAction,
  skillHubCategoryLabel,
  type DiscoverySkill,
} from "./skillsShared";

function DiscoveryCard({
  originRef,
  busy,
  isInstalled,
  skill,
  onInstall,
  onOpenDetail,
}: {
  originRef?: DialogOriginRef;
  busy: boolean;
  isInstalled: boolean;
  skill: DiscoverySkill;
  onInstall: (skill: DiscoverySkill) => void;
  onOpenDetail: (skill: DiscoverySkill) => void;
}) {
  const body = skillCardBody(skill);
  const docs = skillDocsAction(skill);
  const meta = isMarketSkill(skill)
    ? [
        skillHubCategoryLabel(skill.category),
        skill.version ? `v${skill.version}` : null,
        skill.ownerName,
      ]
        .filter(Boolean)
        .join(" · ")
    : "";

  return (
    <article className="fy-feature-card">
      <header className="fy-feature-card-meta">
        <h3>{skill.name}</h3>
        {isInstalled && <Badge tone="accent">已安装</Badge>}
      </header>
      {body ? <p className="fy-feature-card-body">{body}</p> : null}
      {meta ? <p className="fy-feature-card-note">{meta}</p> : null}
      <footer>
        <Button
          className="fy-control-button-primary"
          disabled={busy || isInstalled}
          onClick={() => onInstall(skill)}
          dialogOriginRef={originRef}
        >
          {isInstalled ? "已安装" : "安装"}
        </Button>
        <Button dialogOriginRef={originRef} onClick={() => onOpenDetail(skill)}>
          详情
        </Button>
        {docs ? (
          <ExternalLinkButton url={docs.url}>{docs.label}</ExternalLinkButton>
        ) : null}
      </footer>
    </article>
  );
}

export function SkillDiscovery({
  busy,
  defaultTarget,
  onInstall,
}: {
  busy: boolean;
  defaultTarget: SkillTargetId;
  onInstall: (skill: DiscoverySkill, target: SkillTargetId) => Promise<void>;
}) {
  const originRef = useRef<HTMLElement | null>(null);
  const [search, setSearch] = useState("");
  const [debouncedSearch, setDebouncedSearch] = useState("");
  const [category, setCategory] = useState<SkillHubCategoryFilter>(
    SKILLHUB_CATEGORY_ALL,
  );
  const [page, setPage] = useState(1);
  const [detailSkill, setDetailSkill, detailKey] =
    useDialogState<DiscoverySkill>();
  const [pendingSkill, setPendingSkill, installKey] =
    useDialogState<DiscoverySkill>();
  const resultsTop = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const timer = window.setTimeout(() => {
      setDebouncedSearch(search);
    }, 300);
    return () => window.clearTimeout(timer);
  }, [search]);
  const discoveryQuery = debouncedSearch.trim();
  const installed = useInstalledSkills();
  const market = useSkillHubSearch(discoveryQuery, page, category, true);
  const installedItems = useMemo(() => installed.data ?? [], [installed.data]);
  const skills: DiscoverySkill[] = market.data?.skills ?? [];
  const totalCount = market.data?.totalCount ?? skills.length;
  const totalPages = Math.max(
    1,
    Math.ceil(totalCount / SKILL_DISCOVERY_PAGE_SIZE),
  );
  const currentPage = Math.min(page, totalPages);
  const goToPage = (next: number) => {
    setPage(next);
    resultsTop.current?.scrollIntoView?.({ block: "start" });
  };
  return (
    <section className="fy-feature-workspace" ref={resultsTop}>
      <div className="fy-feature-toolbar">
        <FeatureSearch
          ariaLabel="搜索 Skill 市场"
          placeholder="搜索 Skill 名称或用途"
          value={search}
          onValueChange={(value) => {
            setSearch(value);
            setPage(1);
          }}
        />
        <FeatureTabs
          id="skills-discovery-categories"
          label="分类筛选"
          value={category}
          onChange={(value) => {
            setCategory(value);
            setPage(1);
          }}
          options={SKILLHUB_CATEGORY_TABS}
        />
      </div>
      {installed.error && installed.data !== undefined && (
        <InlineNotice tone="error">
          已安装 Skills 刷新失败，正在显示上一次成功数据：
          {errorMessage(installed.error)}
        </InlineNotice>
      )}
      {market.error && market.data !== undefined && (
        <InlineNotice tone="error">
          Skill 市场刷新失败，正在显示上一次成功数据：
          {errorMessage(market.error)}
        </InlineNotice>
      )}
      {installed.error && installed.data === undefined ? (
        <EmptyState
          title="无法加载已安装 Skills"
          description={errorMessage(installed.error)}
          actions={
            <Button onClick={() => void installed.refetch()}>重试</Button>
          }
        />
      ) : market.error && market.data === undefined ? (
        <EmptyState
          title="Skill 市场搜索失败"
          description={errorMessage(market.error)}
          actions={<Button onClick={() => void market.refetch()}>重试</Button>}
        />
      ) : (installed.data === undefined && installed.isPending) ||
        (market.data === undefined && market.isPending) ? (
        <EmptyState title="正在加载发现内容">
          <Spinner />
        </EmptyState>
      ) : skills.length === 0 ? (
        <EmptyState title="没有发现结果" description="试试其他关键词或分类。" />
      ) : (
        <div className="fy-feature-discovery-scroll" aria-label="可发现 Skills">
          <div className="fy-feature-grid">
            {skills.map((skill) => (
              <DiscoveryCard
                originRef={originRef}
                key={skill.key}
                busy={busy}
                isInstalled={isDiscoverableInstalled(skill, installedItems)}
                skill={skill}
                onInstall={(next) => {
                  setPendingSkill(next);
                }}
                onOpenDetail={setDetailSkill}
              />
            ))}
          </div>
        </div>
      )}
      <FeaturePagination
        page={currentPage}
        totalPages={totalPages}
        ariaLabel="Skill 市场分页"
        onPageChange={goToPage}
      />
      <AnimatePresence>
        {detailSkill ? (
          <Dialog
            key={detailKey}
            originRef={originRef}
            open
            title={detailSkill.name}
            description={skillDetailMeta(detailSkill) || undefined}
            onOpenChange={(open) => {
              if (!open) setDetailSkill(null);
            }}
            actions={<Button onClick={() => setDetailSkill(null)}>关闭</Button>}
          >
            {skillCardBody(detailSkill) && (
              <p className="fy-feature-intro">{skillCardBody(detailSkill)}</p>
            )}
          </Dialog>
        ) : null}
      </AnimatePresence>
      <AnimatePresence>
        {pendingSkill ? (
          <InstallTargetDialog
            key={installKey}
            originRef={originRef}
            title={`安装 ${pendingSkill.name}`}
            busy={busy}
            defaultTarget={defaultTarget}
            pathForTarget={(target) =>
              skillInstallDestination(
                target,
                pendingSkill.directory ||
                  pendingSkill.slug ||
                  pendingSkill.repoName,
              )
            }
            onCancel={() => setPendingSkill(null)}
            onConfirm={(target) => {
              const skill = pendingSkill;
              setPendingSkill(null);
              void onInstall(skill, target);
            }}
          />
        ) : null}
      </AnimatePresence>
    </section>
  );
}

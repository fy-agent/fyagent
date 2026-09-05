import { Children, type CSSProperties, type ReactNode } from "react";

import type { AgentBrandAsset } from "../../assets/agents";
import { classNames } from "../../design-system/classNames";
import { SelectionLens, SelectionLensGroup } from "../SelectionLens";
import { SplitPanes } from "../split";

import "./catalog.css";

const CATALOG_MIN_WIDTHS = [220, 360];
const CATALOG_MAX_WIDTHS = [420];
const CATALOG_SEPARATOR_LABELS = ["调整目录与详情的宽度"];
const CATALOG_PANE_VARS = ["--fy-catalog-rail-width"];

interface CatalogMasterDetailProps {
  children: ReactNode;
  className?: string;
}

export function CatalogMasterDetail({
  children,
  className,
}: CatalogMasterDetailProps) {
  const panes = Children.toArray(children);
  const rail = panes[0];
  const detail = panes.slice(1);

  return (
    <SplitPanes
      className={classNames("fy-catalog-master-detail", className)}
      minWidths={CATALOG_MIN_WIDTHS}
      maxWidths={CATALOG_MAX_WIDTHS}
      separatorLabels={CATALOG_SEPARATOR_LABELS}
      paneCssVars={CATALOG_PANE_VARS}
    >
      {rail}
      {detail.length > 0 ? (
        <div className="fy-catalog-pane">{detail}</div>
      ) : null}
    </SplitPanes>
  );
}

interface CatalogRailProps {
  ariaLabel: string;
  title: string;
  children: ReactNode;
  meta?: ReactNode;
  as?: "aside" | "section";
  className?: string;
}

export function CatalogRail({
  ariaLabel,
  title,
  children,
  meta,
  as = "section",
  className,
}: CatalogRailProps) {
  const content = (
    <>
      <div className="fy-catalog-rail-heading">
        <h2>{title}</h2>
        {meta && <div className="fy-catalog-rail-meta">{meta}</div>}
      </div>
      {children}
    </>
  );
  const classes = classNames("fy-feature-panel", "fy-catalog-rail", className);

  return as === "aside" ? (
    <aside className={classes} aria-label={ariaLabel}>
      {content}
    </aside>
  ) : (
    <section className={classes} aria-label={ariaLabel}>
      {content}
    </section>
  );
}

export function CatalogList({ children }: { children: ReactNode }) {
  return (
    <SelectionLensGroup
      id="catalog-list"
      className="fy-catalog-list"
      role="list"
    >
      {children}
    </SelectionLensGroup>
  );
}

type CatalogBrandFrameSize = "list" | "detail";
type CatalogBrandFrameStyle = CSSProperties & {
  "--fy-catalog-optical-scale": number;
};

interface BrandIconFrameProps {
  asset: AgentBrandAsset;
  size: CatalogBrandFrameSize;
  accessibilityLabel?: string;
}

export function BrandIconFrame({
  asset,
  size,
  accessibilityLabel,
}: BrandIconFrameProps) {
  const optics = asset[size];
  const decorative = accessibilityLabel === undefined;
  const style: CatalogBrandFrameStyle = {
    "--fy-catalog-optical-scale": optics.opticalScale,
  };

  return (
    <span
      className="fy-catalog-brand-frame"
      data-size={size}
      data-background={optics.background}
      data-corner={optics.corner}
      style={style}
    >
      <img
        className="fy-catalog-brand-artwork"
        data-fy-startup-image=""
        src={asset.iconUrl}
        alt={accessibilityLabel ?? ""}
        aria-hidden={decorative ? "true" : undefined}
      />
    </span>
  );
}

interface CatalogListItemProps {
  asset: AgentBrandAsset;
  label: string;
  summary?: ReactNode;
  selected: boolean;
  onSelect: () => void;
  testId?: string;
  disabled?: boolean;
}

export function CatalogListItem({
  asset,
  label,
  summary,
  selected,
  onSelect,
  testId,
  disabled,
}: CatalogListItemProps) {
  return (
    <div role="listitem">
      <button
        type="button"
        className="fy-catalog-list-item"
        aria-current={selected ? "true" : undefined}
        data-testid={testId}
        disabled={disabled}
        onClick={onSelect}
      >
        <SelectionLens active={selected} />
        <BrandIconFrame asset={asset} size="list" />
        <span className="fy-catalog-list-copy">
          <strong>{label}</strong>
          {summary ? <span>{summary}</span> : null}
        </span>
      </button>
    </div>
  );
}

interface CatalogDetailProps {
  ariaLabel: string;
  children: ReactNode;
  className?: string;
}

export function CatalogDetail({
  ariaLabel,
  children,
  className,
}: CatalogDetailProps) {
  return (
    <section
      className={classNames("fy-feature-panel", "fy-catalog-detail", className)}
      aria-label={ariaLabel}
    >
      {children}
    </section>
  );
}

import { GithubLogoIcon } from "@phosphor-icons/react/dist/csr/GithubLogo";
import type { ReactNode } from "react";

import { resolveModelVendorIcon } from "../../shared/assets/models";
import type {
  ManagedAuthProvider,
  ManagedAuthReasonCode,
} from "../../shared/features/managed-auth";
import { SelectionLens } from "../../shared/ui/SelectionLens";
import { Badge } from "../../shared/ui/primitives";
import {
  accountHealthPresentation,
  managedAuthProviderLabel,
  uniqueManagedAuthReasonCopies,
  type AuthTone,
} from "./presentation";

export function ProviderMark({
  provider,
  size = "list",
}: {
  provider: ManagedAuthProvider;
  size?: "list" | "detail";
}) {
  const label = managedAuthProviderLabel(provider);
  if (provider === "github_copilot") {
    return (
      <span className="fy-auth-provider-mark" data-size={size} aria-hidden>
        <GithubLogoIcon weight="fill" />
      </span>
    );
  }
  const iconUrl = resolveModelVendorIcon(
    provider === "openai" ? "gpt" : "grok",
    label,
  );
  return (
    <span className="fy-auth-provider-mark" data-size={size} aria-hidden>
      <img src={iconUrl} alt="" />
    </span>
  );
}

export function StatusBadge({
  label,
  tone,
}: {
  label: string;
  tone: AuthTone;
}) {
  return <Badge tone={tone}>{label}</Badge>;
}

export function AccountHealthBadge({
  health,
}: {
  health: Parameters<typeof accountHealthPresentation>[0];
}) {
  const presentation = accountHealthPresentation(health);
  return <StatusBadge {...presentation} />;
}

export function AuthListItem({
  selected,
  label,
  summary,
  leading,
  trailing,
  onSelect,
  testId,
}: {
  selected: boolean;
  label: string;
  summary: ReactNode;
  leading: ReactNode;
  trailing?: ReactNode;
  onSelect: () => void;
  testId?: string;
}) {
  return (
    <div role="listitem">
      <button
        type="button"
        className="fy-auth-list-item"
        aria-current={selected ? "true" : undefined}
        aria-label={
          typeof summary === "string" ? `${label}，${summary}` : label
        }
        onClick={onSelect}
        data-testid={testId}
      >
        <SelectionLens active={selected} />
        {leading}
        <span className="fy-auth-list-item-copy">
          <strong title={label}>{label}</strong>
          <span>{summary}</span>
        </span>
        {trailing ? (
          <span className="fy-auth-list-item-trailing">{trailing}</span>
        ) : null}
      </button>
    </div>
  );
}

export function DefinitionRow({
  label,
  children,
}: {
  label: string;
  children: ReactNode;
}) {
  return (
    <>
      <dt>{label}</dt>
      <dd>{children}</dd>
    </>
  );
}

export function ReasonList({ reasons }: { reasons: ManagedAuthReasonCode[] }) {
  const copies = uniqueManagedAuthReasonCopies(reasons);
  if (copies.length === 0) return null;
  return (
    <ul className="fy-auth-reason-list">
      {copies.map((copy) => (
        <li key={copy}>{copy}</li>
      ))}
    </ul>
  );
}

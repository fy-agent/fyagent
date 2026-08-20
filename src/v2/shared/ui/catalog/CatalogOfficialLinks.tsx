import type { AgentOfficialLink } from "../../features/types";
import { ExternalLinkButton } from "../ExternalLinkButton";

export function officialLinkActionLabel(link: AgentOfficialLink): string {
  return /官方/.test(link.label) ? link.label : `打开 ${link.label} 官网`;
}

export function CatalogOfficialLinks({
  links,
  disabled = false,
}: {
  links: readonly AgentOfficialLink[];
  disabled?: boolean;
}) {
  if (links.length === 0) return null;

  return (
    <div
      className="fy-catalog-official-links"
      role="group"
      aria-label="官方网站"
    >
      {links.map((link) => (
        <ExternalLinkButton
          key={link.id}
          url={link.url}
          className="fy-control-button-primary"
          disabled={disabled}
          errorTitle="无法打开官方入口"
        >
          {officialLinkActionLabel(link)}
        </ExternalLinkButton>
      ))}
    </div>
  );
}

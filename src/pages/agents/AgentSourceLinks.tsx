import { memo } from "react";

import type { AgentOfficialLink } from "../../shared/features/types";
import { ExternalLinkButton } from "../../shared/features/controls/ExternalLinkButton";
import { resolveAgentSourceLinks } from "../../shared/features/agents";
import "./AgentSourceLinks.css";

export interface AgentSourceLinksProps {
  catalogLinks?: readonly AgentOfficialLink[];
  className?: string;
}

/**
 * Renders verified official homepage, download source, and license/terms links.
 * Uses ExternalLinkButton to open links safely via Tauri.
 * Distinguishes open source software licenses from proprietary terms of service.
 */
export const AgentSourceLinks = memo(function AgentSourceLinks({
  catalogLinks,
  className = "",
}: AgentSourceLinksProps) {
  const links = resolveAgentSourceLinks(catalogLinks);
  if (links.length === 0) return null;

  return (
    <div
      className={`fy-agent-source-links ${className}`.trim()}
      role="region"
      aria-label="官方来源与许可链接"
    >
      <div className="fy-agent-source-links-header">
        <span className="fy-agent-source-links-title">官方来源与许可</span>
      </div>
      <div className="fy-agent-source-links-grid">
        {links.map((link) => (
          <ExternalLinkButton
            key={`${link.category}-${link.id}-${link.url}`}
            url={link.url}
            className="fy-agent-source-link-card"
            title={`${link.label}（在新窗口打开）`}
          >
            <span
              className={`fy-agent-source-link-badge fy-agent-source-link-badge-${link.category}`}
            >
              {link.badge}
            </span>
            <span className="fy-agent-source-link-label">{link.label}</span>
            <span className="fy-agent-source-link-arrow" aria-hidden="true">
              ↗
            </span>
          </ExternalLinkButton>
        ))}
      </div>
    </div>
  );
});


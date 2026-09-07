import { Button } from "../../shared/ui/Button";

export function AgentSectionHeader({
  title,
  description,
  actionLabel,
  onAction,
}: {
  title: string;
  description?: string;
  actionLabel: string;
  onAction: () => void;
}) {
  return (
    <header className="fy-agent-config-section-header">
      <div>
        <h2>{title}</h2>
        {description ? <p>{description}</p> : null}
      </div>
      <Button onClick={onAction}>{actionLabel}</Button>
    </header>
  );
}

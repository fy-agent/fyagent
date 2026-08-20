import markUrl from "../../shared/assets/fyagent-y-mark-transparent-128.png";

export function Brand() {
  return (
    <div
      className="fy-brand"
      role="img"
      aria-label="FyAgent"
      data-testid="brand"
    >
      <img
        className="fy-brand-mark"
        src={markUrl}
        alt=""
        width="28"
        height="28"
      />
      <span className="fy-brand-name">FyAgent</span>
    </div>
  );
}

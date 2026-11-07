import {
  FIRST_INSTALL_STEPS,
  FREE_LANE_NOTICE,
  OPENCODE_UPDATE_NOTICE,
  PARENT_RULE_NOTICE,
  PENDING_EXTERNAL_STEPS,
  PRERELEASE_EXCLUSION_NOTICE,
  WINDOWS_ENV_HINT,
  WINDOWS_LOGIN_SPIN_HINT,
} from "../../domain/first-install-path";
import { InlineNotice } from "../../shared/ui/primitives";

import "./FirstInstallPathPanel.css";

export function FirstInstallPathPanel() {
  return (
    <section className="fy-first-install-path" aria-label="首次安装路径">
      <h3>第一次安装会问什么</h3>
      <ol>
        {FIRST_INSTALL_STEPS.map((step) => (
          <li key={step.step}>
            <p>
              <strong>第 {step.step} 步问什么。</strong>
              {step.asks}
            </p>
            <p>
              <strong>这一步装什么。</strong>
              {step.installs}
              {step.evidence === "pending" ? (
                <span className="fy-first-install-pending">待补证</span>
              ) : null}
            </p>
          </li>
        ))}
      </ol>
      <p>{PENDING_EXTERNAL_STEPS}</p>
      <h3>免费入口</h3>
      <InlineNotice tone="warning">{FREE_LANE_NOTICE}</InlineNotice>
      <h3>这些规则不再有效</h3>
      <ul>
        <li>{OPENCODE_UPDATE_NOTICE}</li>
        <li>{PARENT_RULE_NOTICE}</li>
      </ul>
      <h3>Windows 上卡住时</h3>
      <ul>
        <li>{WINDOWS_LOGIN_SPIN_HINT}</li>
        <li>{WINDOWS_ENV_HINT}</li>
      </ul>
      <p>{PRERELEASE_EXCLUSION_NOTICE}</p>
    </section>
  );
}

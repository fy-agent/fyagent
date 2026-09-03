import React from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { User } from "lucide-react";
import { useCodexOauth } from "./hooks/useCodexOauth";
import CodexOauthAccountQuota from "@/components/CodexOauthAccountQuota";

interface CodexOAuthSectionProps {
  className?: string;
  /** 是否展示每个账号的订阅额度 */
  showAccountQuota?: boolean;
  /** 当前选中的 ChatGPT 账号 ID */
  selectedAccountId?: string | null;
  /** 账号选择回调 */
  onAccountSelect?: (accountId: string | null) => void;
  /** 是否开启 Codex FAST mode */
  fastModeEnabled?: boolean;
  /** FAST mode 切换回调 */
  onFastModeChange?: (enabled: boolean) => void;
}

/**
 * Leftover Codex OAuth picker for Provider `authBinding`.
 * Login, reauth, and removal live on the V2 Accounts & authentication page.
 */
export const CodexOAuthSection: React.FC<CodexOAuthSectionProps> = ({
  className,
  showAccountQuota = false,
  selectedAccountId,
  onAccountSelect,
  fastModeEnabled = false,
  onFastModeChange,
}) => {
  const { t } = useTranslation();
  const { accounts, defaultAccountId, hasAnyAccount, authStatus } =
    useCodexOauth();

  const handleAccountSelect = (value: string) => {
    onAccountSelect?.(value === "none" ? null : value);
  };

  return (
    <div className={`space-y-4 ${className || ""}`}>
      <div className="flex items-center justify-between">
        <Label>{t("codexOauth.authStatus", "认证状态")}</Label>
        <Badge
          variant={hasAnyAccount ? "default" : "secondary"}
          className={hasAnyAccount ? "bg-green-500 hover:bg-green-600" : ""}
        >
          {hasAnyAccount
            ? t("codexOauth.accountCount", {
                count: accounts.length,
                defaultValue: `${accounts.length} 个账号`,
              })
            : t("codexOauth.notAuthenticated", "未认证")}
        </Badge>
      </div>

      <p className="text-xs text-muted-foreground">
        {t("settings.authCenter.providerBindingHint", {
          defaultValue:
            "登录、重新登录和移除账号请到「账号与认证」页面完成。这里只能选择已保存账号用于当前供应商绑定。",
        })}
      </p>

      {authStatus?.native_projection_available === false ? (
        <p className="text-xs text-muted-foreground">
          {t(
            "codexOauth.nativeProjectionUnavailable",
            "Codex 当前不使用 auth.json 保存凭据。请在 Codex 中登录；这里的账号只用于 FyAgent 路由。",
          )}
        </p>
      ) : null}

      {hasAnyAccount && onAccountSelect && (
        <div className="space-y-2">
          <Label className="text-sm text-muted-foreground">
            {t("codexOauth.selectAccount", "选择账号")}
          </Label>
          <Select
            value={selectedAccountId || "none"}
            onValueChange={handleAccountSelect}
          >
            <SelectTrigger>
              <SelectValue
                placeholder={t(
                  "codexOauth.selectAccountPlaceholder",
                  "选择一个 ChatGPT 账号",
                )}
              />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="none">
                <span className="text-muted-foreground">
                  {t("codexOauth.useDefaultAccount", "使用默认账号")}
                </span>
              </SelectItem>
              {accounts.map((account) => (
                <SelectItem key={account.id} value={account.id}>
                  <div className="flex items-center gap-2">
                    <User className="h-4 w-4 text-muted-foreground" />
                    <span>{account.login}</span>
                  </div>
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      )}

      {onFastModeChange && (
        <div className="flex items-center justify-between rounded-md border bg-muted/30 p-3">
          <div className="space-y-1 pr-4">
            <Label className="text-sm font-medium">
              {t("codexOauth.fastMode", "FAST mode")}
            </Label>
            <p className="text-xs text-muted-foreground">
              {t("codexOauth.fastModeDescription", {
                defaultValue:
                  'Send service_tier="priority" for lower latency. Turn it off if the ChatGPT Codex backend rejects the parameter.',
              })}
            </p>
          </div>
          <Switch
            checked={fastModeEnabled}
            onCheckedChange={onFastModeChange}
            aria-label={t("codexOauth.fastMode", "FAST mode")}
          />
        </div>
      )}

      {hasAnyAccount && (
        <div className="space-y-2">
          <Label className="text-sm text-muted-foreground">
            {t("codexOauth.loggedInAccounts", "已登录账号")}
          </Label>
          <div className="space-y-1">
            {accounts.map((account) => (
              <div
                key={account.id}
                className="space-y-2 p-2 rounded-md border bg-muted/30"
              >
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <User className="h-5 w-5 text-muted-foreground" />
                    <span className="text-sm font-medium">{account.login}</span>
                    {defaultAccountId === account.id && (
                      <Badge variant="secondary" className="text-xs">
                        {t("codexOauth.defaultAccount", "默认")}
                      </Badge>
                    )}
                    {selectedAccountId === account.id && (
                      <Badge variant="outline" className="text-xs">
                        {t("codexOauth.selected", "已选中")}
                      </Badge>
                    )}
                  </div>
                </div>
                {showAccountQuota && (
                  <CodexOauthAccountQuota accountId={account.id} />
                )}
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
};

export default CodexOAuthSection;

import React from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { User } from "lucide-react";
import { useCopilotAuth } from "./hooks/useCopilotAuth";
import type { ManagedAuthAccount } from "@/lib/api";

interface CopilotAuthSectionProps {
  className?: string;
  /** 当前选中的 GitHub 账号 ID */
  selectedAccountId?: string | null;
  /** 账号选择回调 */
  onAccountSelect?: (accountId: string | null) => void;
}

/**
 * Leftover Copilot picker for Provider `authBinding`.
 * Login, reauth, and removal live on the V2 Accounts & authentication page.
 */
export const CopilotAuthSection: React.FC<CopilotAuthSectionProps> = ({
  className,
  selectedAccountId,
  onAccountSelect,
}) => {
  const { t } = useTranslation();
  const { accounts, defaultAccountId, migrationError, hasAnyAccount } =
    useCopilotAuth();

  const handleAccountSelect = (value: string) => {
    onAccountSelect?.(value === "none" ? null : value);
  };

  return (
    <div className={`space-y-4 ${className || ""}`}>
      <div className="flex items-center justify-between">
        <Label>{t("copilot.authStatus", "GitHub Copilot 认证")}</Label>
        <Badge
          variant={hasAnyAccount ? "default" : "secondary"}
          className={hasAnyAccount ? "bg-green-500 hover:bg-green-600" : ""}
        >
          {hasAnyAccount
            ? t("copilot.accountCount", {
                count: accounts.length,
                defaultValue: `${accounts.length} 个账号`,
              })
            : t("copilot.notAuthenticated", "未认证")}
        </Badge>
      </div>

      <p className="text-xs text-muted-foreground">
        {t("settings.authCenter.providerBindingHint", {
          defaultValue:
            "登录、重新登录和移除账号请到「账号与认证」页面完成。这里只能选择已保存账号用于当前供应商绑定。",
        })}
      </p>

      {migrationError ? (
        <p className="text-sm text-amber-600 dark:text-amber-400">
          {t("copilot.migrationFailed", {
            defaultValue: "旧认证数据迁移失败。请到「账号与认证」页面处理。",
          })}
        </p>
      ) : null}

      {hasAnyAccount && onAccountSelect && (
        <div className="space-y-2">
          <Label className="text-sm text-muted-foreground">
            {t("copilot.selectAccount", "选择账号")}
          </Label>
          <Select
            value={selectedAccountId || "none"}
            onValueChange={handleAccountSelect}
          >
            <SelectTrigger>
              <SelectValue
                placeholder={t(
                  "copilot.selectAccountPlaceholder",
                  "选择一个 GitHub 账号",
                )}
              />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="none">
                <span className="text-muted-foreground">
                  {t("copilot.useDefaultAccount", "使用默认账号")}
                </span>
              </SelectItem>
              {accounts.map((account) => (
                <SelectItem key={account.id} value={account.id}>
                  <div className="flex items-center gap-2">
                    <CopilotAccountAvatar account={account} />
                    <span>{account.login}</span>
                  </div>
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      )}

      {hasAnyAccount && (
        <div className="space-y-2">
          <Label className="text-sm text-muted-foreground">
            {t("copilot.loggedInAccounts", "已登录账号")}
          </Label>
          <div className="space-y-1">
            {accounts.map((account) => (
              <div
                key={account.id}
                className="flex items-center justify-between p-2 rounded-md border bg-muted/30"
              >
                <div className="flex items-center gap-2">
                  <CopilotAccountAvatar account={account} />
                  <span className="text-sm font-medium">{account.login}</span>
                  {defaultAccountId === account.id && (
                    <Badge variant="secondary" className="text-xs">
                      {t("copilot.defaultAccount", "默认")}
                    </Badge>
                  )}
                  {account.github_domain &&
                    account.github_domain !== "github.com" && (
                      <Badge variant="outline" className="text-xs">
                        {account.github_domain}
                      </Badge>
                    )}
                  {selectedAccountId === account.id && (
                    <Badge variant="outline" className="text-xs">
                      {t("copilot.selected", "已选中")}
                    </Badge>
                  )}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
};

const CopilotAccountAvatar: React.FC<{ account: ManagedAuthAccount }> = ({
  account,
}) => {
  const [failed, setFailed] = React.useState(false);

  if (!account.avatar_url || failed) {
    return <User className="h-5 w-5 text-muted-foreground" />;
  }

  return (
    <img
      src={account.avatar_url}
      alt={account.login}
      className="h-5 w-5 rounded-full"
      loading="lazy"
      referrerPolicy="no-referrer"
      onError={() => setFailed(true)}
    />
  );
};

export default CopilotAuthSection;

import React from "react";
import { useTranslation } from "react-i18next";
import { AlertTriangle, User } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useXaiOauth } from "./hooks/useXaiOauth";

interface XaiOAuthSectionProps {
  className?: string;
  selectedAccountId?: string | null;
  onAccountSelect?: (accountId: string | null) => void;
}

export const XaiOAuthSection: React.FC<XaiOAuthSectionProps> = ({
  className,
  selectedAccountId,
  onAccountSelect,
}) => {
  const { t } = useTranslation();
  const { accounts, defaultAccountId, hasAnyAccount, isAuthenticated } =
    useXaiOauth();

  const usableAccounts = accounts.filter((account) => !account.requires_reauth);

  return (
    <div className={`space-y-4 ${className ?? ""}`}>
      <div className="flex items-center justify-between">
        <Label>{t("xaiOauth.authStatus", "xAI OAuth 认证")}</Label>
        <Badge
          variant={isAuthenticated ? "default" : "secondary"}
          className={
            isAuthenticated
              ? "bg-green-500 hover:bg-green-600"
              : hasAnyAccount
                ? "border-amber-500 text-amber-600"
                : ""
          }
        >
          {isAuthenticated
            ? t("xaiOauth.accountCount", {
                count: usableAccounts.length,
                defaultValue: `${usableAccounts.length} 个可用账号`,
              })
            : hasAnyAccount
              ? t("xaiOauth.reauthRequired", "需要重新登录")
              : t("xaiOauth.notAuthenticated", "未认证")}
        </Badge>
      </div>

      <p className="text-xs text-muted-foreground">
        {t("settings.authCenter.providerBindingHint", {
          defaultValue:
            "登录、重新登录和移除账号请到「账号与认证」页面完成。这里只能选择已保存账号用于当前供应商绑定。",
        })}
      </p>

      {accounts.length > 0 && onAccountSelect && (
        <div className="space-y-2">
          <Label className="text-sm text-muted-foreground">
            {t("xaiOauth.selectAccount", "选择账号")}
          </Label>
          <Select
            value={selectedAccountId || "none"}
            onValueChange={(value) =>
              onAccountSelect(value === "none" ? null : value)
            }
          >
            <SelectTrigger>
              <SelectValue
                placeholder={t(
                  "xaiOauth.selectAccountPlaceholder",
                  "选择 xAI 账号",
                )}
              />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="none">
                {t("xaiOauth.useDefaultAccount", "使用默认账号")}
              </SelectItem>
              {accounts.map((account) => (
                <SelectItem
                  key={account.id}
                  value={account.id}
                  disabled={account.requires_reauth}
                >
                  <span className="flex items-center gap-2">
                    {account.requires_reauth ? (
                      <AlertTriangle className="h-4 w-4 text-amber-500" />
                    ) : (
                      <User className="h-4 w-4 text-muted-foreground" />
                    )}
                    {account.login}
                    {account.requires_reauth && (
                      <span className="text-xs text-amber-600">
                        ({t("xaiOauth.expired", "凭据已失效")})
                      </span>
                    )}
                  </span>
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      )}

      {hasAnyAccount && (
        <div className="space-y-2">
          <Label className="text-sm text-muted-foreground">
            {t("xaiOauth.accounts", "xAI 账号")}
          </Label>
          <div className="space-y-1">
            {accounts.map((account) => (
              <div
                key={account.id}
                className="flex items-center justify-between rounded-md border bg-muted/30 p-2"
              >
                <div className="flex min-w-0 items-center gap-2">
                  {account.requires_reauth ? (
                    <AlertTriangle className="h-5 w-5 shrink-0 text-amber-500" />
                  ) : (
                    <User className="h-5 w-5 shrink-0 text-muted-foreground" />
                  )}
                  <span className="truncate text-sm font-medium">
                    {account.login}
                  </span>
                  {defaultAccountId === account.id && (
                    <Badge variant="secondary" className="text-xs">
                      {t("xaiOauth.defaultAccount", "默认")}
                    </Badge>
                  )}
                  {account.requires_reauth && (
                    <Badge
                      variant="outline"
                      className="border-amber-500 text-xs text-amber-600"
                    >
                      {t("xaiOauth.expired", "凭据已失效")}
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

export default XaiOAuthSection;

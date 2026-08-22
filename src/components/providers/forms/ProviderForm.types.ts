import type { UniversalProviderPreset } from "@/config/universalProviderPresets";
import type { OpenClawSuggestedDefaults } from "@/config/openclawProviderPresets";
import type { AppId } from "@/lib/api";
import type { ProviderFormData } from "@/lib/schemas/provider";
import type { ProviderCategory, ProviderMeta } from "@/types";

export type ProviderFormValues = ProviderFormData & {
  presetId?: string;
  presetCategory?: ProviderCategory;
  meta?: ProviderMeta;
  providerKey?: string;
  suggestedDefaults?: OpenClawSuggestedDefaults;
};

export interface ProviderFormProps {
  appId: AppId;
  providerId?: string;
  submitLabel: string;
  onSubmit: (values: ProviderFormValues) => Promise<void> | void;
  onCancel: () => void;
  onUniversalPresetSelect?: (preset: UniversalProviderPreset) => void;
  onManageUniversalProviders?: () => void;
  onSubmittingChange?: (isSubmitting: boolean) => void;
  initialData?: {
    name?: string;
    websiteUrl?: string;
    notes?: string;
    settingsConfig?: Record<string, unknown>;
    category?: ProviderCategory;
    meta?: ProviderMeta;
    icon?: string;
    iconColor?: string;
  };
  showButtons?: boolean;
  isProxyTakeover?: boolean;
}

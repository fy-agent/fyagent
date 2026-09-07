import type { TemplateValueConfig } from "../presets/claudeProviderPresets";
import { deepClone } from "@/domain/configuration/serialization/deepClone";
import {
  deepMerge,
  deepRemove,
  isPlainObject,
  isSubset,
  sanitizeSnippet,
} from "@/domain/configuration/serialization/providerConfigStructural";

export interface UpdateCommonConfigResult {
  updatedConfig: string;
  error?: string;
}

export const validateJsonConfig = (
  value: string,
  fieldName: string = "配置",
): string => {
  if (!value.trim()) return "";
  try {
    const parsed = JSON.parse(value);
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
      return `${fieldName}必须是 JSON 对象`;
    }
    return "";
  } catch {
    return `${fieldName}JSON格式错误，请检查语法`;
  }
};

export const updateCommonConfigSnippet = (
  jsonString: string,
  snippetString: string,
  enabled: boolean,
): UpdateCommonConfigResult => {
  let config: Record<string, unknown>;
  try {
    const parsed: unknown = jsonString ? JSON.parse(jsonString) : {};
    if (!isPlainObject(parsed))
      throw new Error("Configuration must be an object");
    config = parsed;
  } catch {
    return {
      updatedConfig: jsonString,
      error: "配置 JSON 解析失败，无法应用通用配置",
    };
  }

  if (!snippetString.trim()) {
    return { updatedConfig: JSON.stringify(config, null, 2) };
  }

  const snippetError = validateJsonConfig(snippetString, "通用配置片段");
  if (snippetError) {
    return {
      updatedConfig: JSON.stringify(config, null, 2),
      error: snippetError,
    };
  }

  const snippet: unknown = JSON.parse(snippetString);
  if (!isPlainObject(snippet))
    return { updatedConfig: jsonString, error: "通用配置片段必须是 JSON 对象" };
  if (enabled) {
    return {
      updatedConfig: JSON.stringify(
        deepMerge(deepClone(config), snippet),
        null,
        2,
      ),
    };
  }

  const cloned = deepClone(config);
  deepRemove(cloned, snippet);
  return { updatedConfig: JSON.stringify(cloned, null, 2) };
};

export const hasCommonConfigSnippet = (
  jsonString: string,
  snippetString: string,
): boolean => {
  try {
    if (!snippetString.trim()) return false;
    const config = jsonString ? JSON.parse(jsonString) : {};
    const parsed = JSON.parse(snippetString);
    if (!isPlainObject(parsed)) return false;
    const snippet = sanitizeSnippet(parsed);
    if (Object.keys(snippet).length === 0) return false;
    return isSubset(config, snippet);
  } catch {
    return false;
  }
};

export const getApiKeyFromConfig = (
  jsonString: string,
  appType?: string,
): string => {
  try {
    const config = JSON.parse(jsonString);
    if (
      typeof config?.apiKey === "string" &&
      config.apiKey &&
      !config.apiKey.includes("${")
    ) {
      return config.apiKey;
    }

    const env = config?.env;
    if (!env) return "";
    if (appType === "gemini") {
      return typeof env.GEMINI_API_KEY === "string" ? env.GEMINI_API_KEY : "";
    }
    if (appType === "codex") {
      return typeof env.CODEX_API_KEY === "string" ? env.CODEX_API_KEY : "";
    }

    const token = env.ANTHROPIC_AUTH_TOKEN;
    const apiKey = env.ANTHROPIC_API_KEY;
    return typeof token === "string"
      ? token
      : typeof apiKey === "string"
        ? apiKey
        : "";
  } catch {
    return "";
  }
};

export const applyTemplateValues = (
  config: unknown,
  templateValues: Record<string, TemplateValueConfig> | undefined,
): unknown => {
  const resolvedValues = Object.fromEntries(
    Object.entries(templateValues ?? {}).map(([key, value]) => {
      const resolvedValue =
        value.editorValue !== undefined
          ? value.editorValue
          : (value.defaultValue ?? "");
      return [key, resolvedValue];
    }),
  );

  const replaceInString = (str: string): string =>
    Object.entries(resolvedValues).reduce((acc, [key, value]) => {
      const placeholder = `\${${key}}`;
      return acc.includes(placeholder)
        ? acc.split(placeholder).join(value ?? "")
        : acc;
    }, str);

  const traverse = (obj: unknown): unknown => {
    if (typeof obj === "string") return replaceInString(obj);
    if (Array.isArray(obj)) return obj.map(traverse);
    if (obj && typeof obj === "object") {
      const result: Record<string, unknown> = {};
      for (const [key, value] of Object.entries(obj))
        Object.defineProperty(result, key, {
          value: traverse(value),
          enumerable: true,
          writable: true,
          configurable: true,
        });
      return result;
    }
    return obj;
  };

  return traverse(config);
};

export const hasApiKeyField = (
  jsonString: string,
  appType?: string,
): boolean => {
  try {
    const config = JSON.parse(jsonString);
    if (Object.prototype.hasOwnProperty.call(config, "apiKey")) return true;
    const env = config?.env ?? {};
    if (appType === "gemini") {
      return Object.prototype.hasOwnProperty.call(env, "GEMINI_API_KEY");
    }
    if (appType === "codex") {
      return Object.prototype.hasOwnProperty.call(env, "CODEX_API_KEY");
    }
    return (
      Object.prototype.hasOwnProperty.call(env, "ANTHROPIC_AUTH_TOKEN") ||
      Object.prototype.hasOwnProperty.call(env, "ANTHROPIC_API_KEY")
    );
  } catch {
    return false;
  }
};

export const setApiKeyInConfig = (
  jsonString: string,
  apiKey: string,
  options: {
    createIfMissing?: boolean;
    appType?: string;
    apiKeyField?: string;
  } = {},
): string => {
  const { createIfMissing = false, appType, apiKeyField } = options;
  try {
    const config = JSON.parse(jsonString);
    if (Object.prototype.hasOwnProperty.call(config, "apiKey")) {
      config.apiKey = apiKey;
      return JSON.stringify(config, null, 2);
    }

    if (!config.env) {
      if (!createIfMissing) return jsonString;
      config.env = {};
    }
    const env: unknown = config.env;
    if (!isPlainObject(env)) return jsonString;

    if (appType === "gemini") {
      if ("GEMINI_API_KEY" in env || createIfMissing) {
        env.GEMINI_API_KEY = apiKey;
      } else {
        return jsonString;
      }
      return JSON.stringify(config, null, 2);
    }

    if (appType === "codex") {
      if ("CODEX_API_KEY" in env || createIfMissing) {
        env.CODEX_API_KEY = apiKey;
      } else {
        return jsonString;
      }
      return JSON.stringify(config, null, 2);
    }

    if ("ANTHROPIC_AUTH_TOKEN" in env) {
      env.ANTHROPIC_AUTH_TOKEN = apiKey;
    } else if ("ANTHROPIC_API_KEY" in env) {
      env.ANTHROPIC_API_KEY = apiKey;
    } else if (createIfMissing) {
      env[apiKeyField ?? "ANTHROPIC_AUTH_TOKEN"] = apiKey;
    } else {
      return jsonString;
    }
    return JSON.stringify(config, null, 2);
  } catch {
    return jsonString;
  }
};

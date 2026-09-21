import { useState } from "react";
import {
  API_PROTOCOL_LABELS,
  PROVIDER_API_PRESETS,
  apiProtocolsForTarget,
  isApiProtocol,
  presetConnection,
  presetForEndpoint,
  type ApiConnection,
  type ApiConfigTarget,
  type ApiProtocol,
} from "../../../domain/configuration/providerApi";
import type { ProviderSummary } from "../models";
import { Button } from "../../ui/Button";
import { InlineNotice } from "../../ui/primitives";
import { ExternalLinkButton } from "./ExternalLinkButton";

export interface ProviderApiFormFill {
  name: string;
  connection: ApiConnection;
}

/** Explicit form filling only. This control owns no credential or mutation. */
export function ProviderApiFields({
  app,
  baseUrl,
  protocol,
  saved,
  disabled,
  error,
  onFill,
  onProtocolChange,
}: {
  app: ApiConfigTarget;
  baseUrl: string;
  protocol: ApiProtocol;
  saved: readonly ProviderSummary[];
  disabled: boolean;
  error?: string;
  onFill: (value: ProviderApiFormFill) => void;
  onProtocolChange: (protocol: ApiProtocol) => void;
}) {
  const [savedId, setSavedId] = useState("");
  const protocols = apiProtocolsForTarget(app);
  const selectedPreset = presetForEndpoint(baseUrl);
  const fillable = saved.filter(
    (item) => item.connection && protocols.includes(item.connection.protocol),
  );
  const selectedSaved = fillable.find((item) => item.id === savedId);
  return (
    <>
      <div className="fy-control-field">
        <label htmlFor={`${app}-api-preset`}>服务商预设</label>
        <select
          id={`${app}-api-preset`}
          className="fy-control-select"
          value=""
          disabled={disabled}
          onChange={(event) => {
            const preset = PROVIDER_API_PRESETS.find(
              (item) => item.id === event.target.value,
            );
            const connection = preset && presetConnection(preset, app);
            if (preset && connection) onFill({ name: preset.name, connection });
          }}
        >
          <option value="">选择预设填入，或自行填写</option>
          {PROVIDER_API_PRESETS.filter((preset) =>
            presetConnection(preset, app),
          ).map((preset) => (
            <option key={preset.id} value={preset.id}>
              {preset.name}
            </option>
          ))}
        </select>
        <small>
          选择后填入地址、模型和协议，并清空当前 Key；不会连接或保存。
        </small>
      </div>
      <div className="fy-control-field">
        <label htmlFor={`${app}-api-protocol`}>API 协议</label>
        <select
          id={`${app}-api-protocol`}
          className="fy-control-select"
          value={protocol}
          disabled={disabled}
          aria-invalid={Boolean(error)}
          aria-describedby={error ? `${app}-api-protocol-error` : undefined}
          onChange={(event) => {
            if (
              isApiProtocol(event.target.value) &&
              protocols.includes(event.target.value)
            )
              onProtocolChange(event.target.value);
          }}
        >
          {protocols.map((value) => (
            <option key={value} value={value}>
              {API_PROTOCOL_LABELS[value]}
            </option>
          ))}
        </select>
        {error && (
          <span
            id={`${app}-api-protocol-error`}
            role="alert"
            className="fy-control-field-error"
          >
            {error}
          </span>
        )}
        {app === "codex" && protocol === "chat" && (
          <InlineNotice tone="warning">
            Chat 需要兼容该协议的 Codex 客户端或已有的协议转换配置；新版 Codex
            直连可能不支持。保存不会自动开启代理。生图扩展和 WebSocket 仅用于
            Responses。
          </InlineNotice>
        )}
      </div>
      {selectedPreset && (
        <div className="fy-control-field">
          <strong>
            {selectedPreset.name} · {selectedPreset.region}
          </strong>
          <small>{selectedPreset.keyScope}</small>
          <small>{selectedPreset.usage}</small>
          <ExternalLinkButton url={selectedPreset.docsUrl}>
            查看官方接入说明
          </ExternalLinkButton>
        </div>
      )}
      {fillable.length > 0 && (
        <div className="fy-control-field">
          <label htmlFor={`${app}-saved-api-config`}>从已保存配置填入</label>
          <select
            id={`${app}-saved-api-config`}
            className="fy-control-select"
            value={savedId}
            disabled={disabled}
            onChange={(event) => setSavedId(event.target.value)}
          >
            <option value="">选择配置</option>
            {fillable.map((item) => (
              <option key={item.id} value={item.id}>
                {item.name}
              </option>
            ))}
          </select>
          <Button
            disabled={disabled || !selectedSaved?.connection}
            onClick={() => {
              if (selectedSaved?.connection)
                onFill({
                  name: selectedSaved.name,
                  connection: selectedSaved.connection,
                });
            }}
          >
            填入表单
          </Button>
          <small>
            只回填名称、地址、模型与协议，需重新填写
            Key。保存会更新本页配置；切换已保存 Codex 来源请进入账号与来源。
          </small>
        </div>
      )}
    </>
  );
}

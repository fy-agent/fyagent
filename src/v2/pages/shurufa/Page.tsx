import { useEffect, useRef, useState } from "react";

import { errorMessage, isNativeOnlyError } from "../../shared/features/helpers";
import { useFeatures } from "../../shared/features/provider";
import type { ShurufaConfig } from "../../shared/features/ports";
import { CopyablePath } from "../../shared/ui/CopyablePath";
import {
  Button,
  InlineNotice,
  Input,
  SecretInput,
  Spinner,
} from "../../shared/ui/primitives";

import "./page.css";

type ConfigDraft = Omit<ShurufaConfig, "configured">;

const EMPTY_CONFIG: ConfigDraft = {
  url: "https://api.openai.com/v1",
  model: "gpt-4o-mini",
  apiKey: "",
  maxSummaries: 8,
  timeoutSecs: 60,
};

export function ShurufaPage() {
  const { ports, notify } = useFeatures();
  const [prompt, setPrompt] = useState("");
  const [config, setConfig] = useState<ConfigDraft>(EMPTY_CONFIG);
  const [configured, setConfigured] = useState(false);
  const [running, setRunning] = useState(false);
  const [output, setOutput] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [shortcutLabel, setShortcutLabel] = useState("⌘M");
  const [dataDir, setDataDir] = useState("");
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const outputRef = useRef("");
  const nativeUnavailable = isNativeOnlyError(error);

  useEffect(() => {
    let cancelled = false;
    void ports.shurufa
      .getSnapshot()
      .then((snapshot) => {
        if (cancelled) return;
        setPrompt(snapshot.prompt);
        setConfig({
          url: snapshot.config.url,
          model: snapshot.config.model,
          apiKey: snapshot.config.apiKey,
          maxSummaries: snapshot.config.maxSummaries,
          timeoutSecs: snapshot.config.timeoutSecs,
        });
        setConfigured(snapshot.config.configured);
        setRunning(snapshot.running);
        outputRef.current = snapshot.lastOutput;
        setOutput(snapshot.lastOutput);
        setError(snapshot.lastError);
        setShortcutLabel(snapshot.shortcutLabel);
        setDataDir(snapshot.dataDir);
      })
      .catch((cause) => {
        if (!cancelled) setError(errorMessage(cause));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [ports.shurufa]);

  useEffect(() => {
    let cancelled = false;
    let unlisten = () => {};
    void ports.shurufa
      .subscribe((event) => {
        if (cancelled) return;
        if (event.type === "started") {
          outputRef.current = "";
          setRunning(true);
          setOutput("");
          setError(null);
          return;
        }
        if (event.type === "delta") {
          outputRef.current += event.text;
          setOutput(outputRef.current);
          return;
        }
        if (event.type === "finished") {
          outputRef.current = event.output;
          setRunning(false);
          setOutput(event.output);
          return;
        }
        setRunning(false);
        setError(event.message);
      })
      .then((stop) => {
        if (cancelled) {
          stop();
          return;
        }
        unlisten = stop;
      })
      .catch((cause) => {
        setError(errorMessage(cause));
      });
    return () => {
      cancelled = true;
      unlisten();
    };
  }, [ports.shurufa]);

  const updatePrompt = (text: string) => {
    setPrompt(text);
    void ports.shurufa.setPrompt(text).catch((cause) => {
      setError(errorMessage(cause));
    });
  };

  const saveConfig = async () => {
    setSaving(true);
    try {
      const next = await ports.shurufa.saveConfig(config);
      setConfig({
        url: next.url,
        model: next.model,
        apiKey: next.apiKey,
        maxSummaries: next.maxSummaries,
        timeoutSecs: next.timeoutSecs,
      });
      setConfigured(next.configured);
      setError(null);
      notify({
        tone: next.configured ? "success" : "info",
        title: next.configured ? "配置已保存" : "配置已写入，但仍不完整",
        description: next.configured
          ? "快捷键会使用这份配置调用输入法 Agent"
          : "请检查 url、model 和 api_key 是否都已填好",
      });
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setSaving(false);
    }
  };

  const runPreview = async () => {
    setError(null);
    try {
      await ports.shurufa.run();
    } catch (cause) {
      setError(errorMessage(cause));
    }
  };

  const clearSession = async () => {
    try {
      const deleted = await ports.shurufa.clearSession();
      notify({
        tone: "success",
        title: "已清空会话摘要",
        description: `删除了 ${deleted} 条历史`,
      });
    } catch (cause) {
      setError(errorMessage(cause));
    }
  };

  return (
    <div className="fy-feature-page fy-shurufa-page" data-testid="shurufa-page">
      <header className="fy-feature-header">
        <div>
          <h1 className="fy-shurufa-page-title">输入法</h1>
          <p className="fy-feature-description">
            先在下面写下本轮文本，再把光标放到目标输入框，按下 {shortcutLabel}{" "}
            会流式写入优化后的提示词。
          </p>
        </div>
        <div className="fy-feature-actions">
          <Button onClick={() => void clearSession()} disabled={running}>
            清空会话
          </Button>
          <Button
            className="fy-control-button-primary"
            onClick={() => void runPreview()}
            disabled={running || loading}
          >
            {running ? "生成中…" : "预览生成"}
          </Button>
        </div>
      </header>

      {nativeUnavailable ? (
        <InlineNotice tone="warning">
          输入法 Demo 只在 FyAgent 桌面应用中可用。
        </InlineNotice>
      ) : null}
      {error && !nativeUnavailable ? (
        <InlineNotice tone="error">{error}</InlineNotice>
      ) : null}

      {loading ? (
        <div className="fy-shurufa-loading">
          <Spinner label="加载输入法配置" />
        </div>
      ) : (
        <div className="fy-shurufa-workspace">
          <label className="fy-control-field fy-shurufa-prompt-field">
            本轮文本
            <textarea
              className="fy-control-textarea fy-shurufa-prompt"
              value={prompt}
              onChange={(event) => updatePrompt(event.target.value)}
              placeholder="例如：把登录按钮改成主色，点了要有 loading"
              rows={8}
            />
          </label>

          <section className="fy-shurufa-output" aria-label="生成预览">
            <div className="fy-shurufa-output-head">
              <h2>生成预览</h2>
              {running ? <Spinner label="正在生成" /> : null}
            </div>
            <pre className="fy-shurufa-output-body">
              {output || (running ? "正在等待模型输出…" : "还没有生成内容")}
            </pre>
          </section>

          <section className="fy-shurufa-config" aria-label="模型配置">
            <div className="fy-shurufa-config-head">
              <div>
                <h2>模型配置</h2>
                <p>
                  与 CLI 的 config.toml 相同：url、model、api_key、历史摘要条数、超时。
                  {configured ? " 当前配置可用。" : " 保存后才能调用模型。"}
                </p>
              </div>
              <Button
                className="fy-control-button-primary"
                onClick={() => void saveConfig()}
                disabled={saving || running}
              >
                {saving ? "保存中…" : "保存配置"}
              </Button>
            </div>

            <div className="fy-shurufa-config-grid">
              <label className="fy-control-field">
                API 地址
                <Input
                  value={config.url}
                  onChange={(event) =>
                    setConfig((current) => ({
                      ...current,
                      url: event.target.value,
                    }))
                  }
                  placeholder="https://api.openai.com/v1"
                />
              </label>
              <label className="fy-control-field">
                模型
                <Input
                  value={config.model}
                  onChange={(event) =>
                    setConfig((current) => ({
                      ...current,
                      model: event.target.value,
                    }))
                  }
                  placeholder="gpt-4o-mini"
                />
              </label>
              <label className="fy-control-field fy-shurufa-config-span">
                API Key
                <SecretInput
                  value={config.apiKey}
                  onChange={(event) =>
                    setConfig((current) => ({
                      ...current,
                      apiKey: event.target.value,
                    }))
                  }
                  placeholder="sk-..."
                  autoComplete="off"
                />
              </label>
              <label className="fy-control-field">
                历史摘要条数
                <Input
                  type="number"
                  min={1}
                  max={32}
                  value={config.maxSummaries}
                  onChange={(event) =>
                    setConfig((current) => ({
                      ...current,
                      maxSummaries: Number(event.target.value) || 0,
                    }))
                  }
                />
              </label>
              <label className="fy-control-field">
                超时（秒）
                <Input
                  type="number"
                  min={1}
                  value={config.timeoutSecs}
                  onChange={(event) =>
                    setConfig((current) => ({
                      ...current,
                      timeoutSecs: Number(event.target.value) || 0,
                    }))
                  }
                />
              </label>
            </div>

            {dataDir ? (
              <CopyablePath value={dataDir} label="配置目录" />
            ) : null}
          </section>
        </div>
      )}
    </div>
  );
}

import { useEffect, useState } from "react";
import type { GlobalProxyConfig } from "../../shared/features/settings";
import { useFeatures } from "../../shared/features/provider";
import { Button } from "../../shared/ui/Button";
import { Dialog } from "../../shared/ui/Dialog";
import type { DialogOriginRef } from "../../shared/ui/dialogOrigin";
import { Input, SecretInput } from "../../shared/ui/primitives";
import "./proxy-settings.css";

export default function ProxySettingsDialog({
  originRef,
  onClose,
}: {
  originRef: DialogOriginRef;
  onClose: () => void;
}) {
  const { ports } = useFeatures();
  const [config, setConfig] = useState<GlobalProxyConfig | null>(null);
  const [key, setKey] = useState<string | null>(null);
  const [busy, setBusy] = useState(true);
  const [message, setMessage] = useState("");

  useEffect(() => {
    let active = true;
    void Promise.all([
      ports.settings.getProxyConfig(),
      ports.settings.getProxyAccessKey(),
    ])
      .then(([saved, accessKey]) => {
        if (active) {
          setConfig(saved);
          setKey(accessKey);
        }
      })
      .catch(() => {
        if (active) setMessage("无法读取代理设置，请关闭后重试。");
      })
      .finally(() => {
        if (active) setBusy(false);
      });
    return () => {
      active = false;
    };
  }, [ports]);

  const save = async () => {
    if (!config) return;
    setBusy(true);
    setMessage("");
    try {
      await ports.settings.saveProxyConfig(config);
      const [saved, accessKey] = await Promise.all([
        ports.settings.getProxyConfig(),
        ports.settings.getProxyAccessKey(),
      ]);
      setConfig(saved);
      setKey(accessKey);
      setMessage("代理设置已保存。");
    } catch {
      setMessage("无法保存代理设置，请检查监听地址和端口后重试。");
    } finally {
      setBusy(false);
    }
  };
  const regenerate = async () => {
    setBusy(true);
    setMessage("");
    try {
      setKey(await ports.settings.regenerateProxyAccessKey());
      setMessage("新密钥已保存，旧密钥已失效。请更新局域网客户端。");
    } catch {
      setMessage("无法重新生成密钥，请重试。");
    } finally {
      setBusy(false);
    }
  };
  const copy = async () => {
    if (!key) return;
    try {
      await navigator.clipboard.writeText(key);
      setMessage("访问密钥已复制。");
    } catch {
      setMessage("无法复制密钥，请显示后手动复制。");
    }
  };

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open) onClose();
      }}
      originRef={originRef}
      title="代理设置"
      description="设置本机代理的监听地址与局域网访问密钥。"
      actions={
        <>
          <Button disabled={busy || !config} onClick={() => void save()}>
            保存并重启服务
          </Button>
          <Button onClick={onClose}>关闭</Button>
        </>
      }
    >
      <div className="fy-proxy-settings">
        {config ? (
          <>
            <label htmlFor="proxy-listen-address">监听地址</label>
            <Input
              id="proxy-listen-address"
              value={config.listenAddress}
              disabled={busy}
              onChange={(event) =>
                setConfig({ ...config, listenAddress: event.target.value })
              }
            />
            <p>
              127.0.0.1 仅供本机使用；0.0.0.0
              允许局域网连接，保存时自动生成访问密钥。
            </p>
            <label htmlFor="proxy-listen-port">监听端口</label>
            <Input
              id="proxy-listen-port"
              type="number"
              min={0}
              max={65535}
              value={config.listenPort}
              disabled={busy}
              onChange={(event) =>
                setConfig({ ...config, listenPort: Number(event.target.value) })
              }
            />
            <label htmlFor="proxy-access-key">局域网访问密钥</label>
            <SecretInput
              id="proxy-access-key"
              readOnly
              value={key ?? ""}
              disabled={busy}
              revealLabel="显示访问密钥"
              hideLabel="隐藏访问密钥"
              placeholder="开启局域网监听后自动生成"
            />
            <div className="fy-proxy-settings-actions">
              <Button disabled={busy || !key} onClick={() => void copy()}>
                复制密钥
              </Button>
              <Button disabled={busy} onClick={() => void regenerate()}>
                重新生成密钥
              </Button>
            </div>
            <p>
              局域网客户端填写本机局域网 IP 和监听端口，将此密钥填入 API
              Key；请求头使用 Authorization: Bearer &lt;密钥&gt; 或 x-api-key:
              &lt;密钥&gt;。本机回环访问无需此密钥。
            </p>
            <p>重新生成后旧密钥立即失效。</p>
          </>
        ) : busy ? (
          <p role="status">正在读取代理设置…</p>
        ) : null}
        {message ? <p role="status">{message}</p> : null}
      </div>
    </Dialog>
  );
}

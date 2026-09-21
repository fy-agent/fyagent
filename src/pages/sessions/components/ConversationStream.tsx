import { useState } from "react";
import { CopyIcon } from "@phosphor-icons/react/dist/csr/Copy";
import { CheckIcon } from "@phosphor-icons/react/dist/csr/Check";
import { WarningIcon } from "@phosphor-icons/react/dist/csr/Warning";
import { InfoIcon } from "@phosphor-icons/react/dist/csr/Info";

import type {
  MigratableMessage,
  SessionMessage,
} from "../../../shared/features/session-migration";

export interface ConversationTurn {
  turnNumber: number;
  userMessage?: MigratableMessage;
  assistantMessage?: MigratableMessage;
  isIndeterminate?: boolean;
  isIncomplete?: boolean;
}

export interface ConversationStreamProps {
  turns: ConversationTurn[];
  rawMessages?: SessionMessage[];
  isRawFallback?: boolean;
}

function CodeBlock({ code, language }: { code: string; language?: string }) {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(code);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // Fallback if clipboard API is restricted
    }
  };

  return (
    <div className="fy-code-block-wrapper">
      <div className="fy-code-block-header">
        <span className="fy-code-block-lang">{language || "text"}</span>
        <button
          type="button"
          className="fy-code-copy-btn"
          onClick={handleCopy}
          aria-label="复制代码"
        >
          {copied ? (
            <>
              <CheckIcon size={12} weight="bold" />
              <span>已复制</span>
            </>
          ) : (
            <>
              <CopyIcon size={12} weight="regular" />
              <span>复制</span>
            </>
          )}
        </button>
      </div>
      <pre className="fy-code-content">
        <code>{code}</code>
      </pre>
    </div>
  );
}

function MessageText({ text }: { text: string }) {
  // Simple markdown-like code block extraction
  const parts = text.split(/(```[\s\S]*?```)/g);

  return (
    <div className="fy-message-body-text">
      {parts.map((part, index) => {
        if (part.startsWith("```") && part.endsWith("```")) {
          const lines = part.slice(3, -3).trim().split("\n");
          const firstLine = lines[0]?.trim() || "";
          const isLang = /^[a-zA-Z0-9_-]+$/.test(firstLine);
          const lang = isLang ? firstLine : undefined;
          const code = isLang ? lines.slice(1).join("\n") : lines.join("\n");
          return <CodeBlock key={index} code={code} language={lang} />;
        }
        return (
          <div key={index} className="fy-text-paragraph">
            {part.split("\n").map((line, lIdx) => (
              <span key={lIdx}>
                {line}
                {lIdx < part.split("\n").length - 1 && <br />}
              </span>
            ))}
          </div>
        );
      })}
    </div>
  );
}

export function ConversationStream({
  turns,
  rawMessages,
  isRawFallback,
}: ConversationStreamProps) {
  // If raw fallback mode, display raw messages with their authentic roles
  if (isRawFallback && rawMessages && rawMessages.length > 0) {
    return (
      <div
        className="fy-conversation-stream"
        role="feed"
        aria-label="原始会话流"
      >
        <div
          className="fy-raw-fallback-banner"
          style={{
            padding: "10px 14px",
            marginBottom: "16px",
            background: "var(--fy-surface-soft)",
            border: "1px solid var(--fy-border)",
            borderRadius: "8px",
            fontSize: "12px",
            color: "var(--fy-text-secondary)",
            display: "flex",
            alignItems: "center",
            gap: "8px",
          }}
        >
          <InfoIcon size={16} />
          <span>
            当前展示本地原始消息记录（未生成迁移提取包）。角色按实际记录呈现，不可直接导出。
          </span>
        </div>

        {rawMessages.map((msg, idx) => {
          const isUser = msg.role === "user";
          return (
            <article
              key={idx}
              className={`fy-message-card ${isUser ? "fy-message-user" : "fy-message-assistant"}`}
            >
              <header className="fy-message-header">
                <div
                  className={`fy-message-role-tag ${isUser ? "role-user" : "role-assistant"}`}
                >
                  <span>
                    {isUser
                      ? "用户发问"
                      : msg.role
                        ? `模型回复 (${msg.role})`
                        : "回复"}
                  </span>
                  <span className="fy-turn-pill">#{idx + 1}</span>
                </div>
                {msg.ts && (
                  <time
                    className="fy-message-time"
                    dateTime={new Date(msg.ts).toISOString()}
                  >
                    {new Date(msg.ts).toLocaleTimeString()}
                  </time>
                )}
              </header>
              <div className="fy-message-content">
                <MessageText text={msg.content} />
              </div>
            </article>
          );
        })}
      </div>
    );
  }

  if (turns.length === 0) {
    return (
      <div className="fy-conversation-empty">
        <p>暂无问答消息</p>
      </div>
    );
  }

  return (
    <div className="fy-conversation-stream" role="feed" aria-label="对话问答流">
      {turns.map((turn) => (
        <div className="fy-turn-container" key={turn.turnNumber}>
          {/* 用户提示词卡片 */}
          {turn.userMessage && (
            <article className="fy-message-card fy-message-user">
              <header className="fy-message-header">
                <div className="fy-message-role-tag role-user">
                  <span>用户提示词</span>
                  <span className="fy-turn-pill">第 {turn.turnNumber} 轮</span>
                </div>
                {turn.userMessage.ts && (
                  <time
                    className="fy-message-time"
                    dateTime={new Date(turn.userMessage.ts).toISOString()}
                  >
                    {new Date(turn.userMessage.ts).toLocaleTimeString()}
                  </time>
                )}
              </header>
              <div className="fy-message-content">
                <MessageText text={turn.userMessage.text} />
              </div>
            </article>
          )}

          {/* AI 最终答复卡片 / 未判定警告 / 未完成占位 */}
          {turn.isIndeterminate ? (
            <article className="fy-message-card fy-message-indeterminate">
              <header className="fy-message-header">
                <div className="fy-message-role-tag role-indeterminate">
                  <WarningIcon size={14} weight="bold" />
                  <span>最终答复待判定（无法确认是否为最终答复）</span>
                </div>
              </header>
              <div className="fy-indeterminate-notice">
                <p>
                  该轮次模型输出包含未完成状态、进行中的工具活动或中途终止，未能产生明确可信的最终答复。
                </p>
                <div className="fy-indeterminate-contract-alert">
                  <strong>提示：</strong>
                  为防止未完结或损坏的数据渗入目标系统，禁止导出或恢复包含该轮次的会话。
                </div>
              </div>
            </article>
          ) : turn.isIncomplete ? (
            <article className="fy-message-card fy-message-incomplete">
              <header className="fy-message-header">
                <div className="fy-message-role-tag role-incomplete">
                  <span>轮次未完成 · 等待模型答复中被中断</span>
                </div>
              </header>
              <div className="fy-incomplete-notice">
                <p>该轮次仅记录了用户发出的提示词，模型尚未生成最终答复。</p>
                <span className="fy-incomplete-footnote">
                  系统仅保留用户提问原文。导入目标软件后
                  <strong>不会自动代跑旧指令或继续生成</strong>
                  ，需由您在目标软件中主动发送新消息。
                </span>
              </div>
            </article>
          ) : turn.assistantMessage ? (
            <article className="fy-message-card fy-message-assistant">
              <header className="fy-message-header">
                <div className="fy-message-role-tag role-assistant">
                  <span>AI 最终答复原文</span>
                  <span className="fy-final-verified-badge">Final Answer</span>
                </div>
                {turn.assistantMessage.ts && (
                  <time
                    className="fy-message-time"
                    dateTime={new Date(turn.assistantMessage.ts).toISOString()}
                  >
                    {new Date(turn.assistantMessage.ts).toLocaleTimeString()}
                  </time>
                )}
              </header>
              <div className="fy-message-content">
                <MessageText text={turn.assistantMessage.text} />
              </div>
            </article>
          ) : null}
        </div>
      ))}
    </div>
  );
}
